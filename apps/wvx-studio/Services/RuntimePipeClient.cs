using System.IO.Pipes;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using WvxStudio.Models;

namespace WvxStudio.Services;

public sealed class RuntimePipeClient
{
    private const string PipeName = "wvx-runtime-v1";
    private const int MessageLimitBytes = 64 * 1024;
    private static readonly TimeSpan RequestTimeout = TimeSpan.FromSeconds(3);
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true,
        DefaultIgnoreCondition = JsonIgnoreCondition.Never
    };

    public async Task<RuntimeStatus> GetSettingsAsync(CancellationToken cancellationToken = default)
    {
        var response = await SendAsync(new { version = 1, type = "get_settings" }, cancellationToken);
        EnsureSuccess(response);
        if (response.Settings is null)
            throw new InvalidDataException("Runtime response did not include settings.");

        response.Settings.Validate();
        return new RuntimeStatus(response.Runtime ?? "running", response.Settings);
    }

    public async Task SetSettingsAsync(RuntimeSettings settings, CancellationToken cancellationToken = default)
    {
        settings.Validate();
        var response = await SendAsync(new
        {
            version = 1,
            type = "set_settings",
            settings
        }, cancellationToken);
        EnsureSuccess(response);
    }

    private static async Task<RuntimeResponse> SendAsync(object request, CancellationToken cancellationToken)
    {
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(RequestTimeout);
        await using var pipe = new NamedPipeClientStream(
            ".", PipeName, PipeDirection.InOut, PipeOptions.Asynchronous);
        try
        {
            await pipe.ConnectAsync(timeout.Token);
            var payload = JsonSerializer.Serialize(request, JsonOptions);
            var bytes = Encoding.UTF8.GetBytes(payload + "\n");
            if (bytes.Length > MessageLimitBytes)
                throw new InvalidDataException("Request exceeds the runtime message limit.");

            await pipe.WriteAsync(bytes, timeout.Token);
            await pipe.FlushAsync(timeout.Token);
            var responseLine = await ReadLineAsync(pipe, timeout.Token);
            return JsonSerializer.Deserialize<RuntimeResponse>(responseLine, JsonOptions)
                ?? throw new InvalidDataException("Runtime returned an empty response.");
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
            throw new TimeoutException($"Runtime did not respond within {RequestTimeout.TotalSeconds:0} seconds.");
        }
    }

    private static async Task<string> ReadLineAsync(Stream stream, CancellationToken cancellationToken)
    {
        var buffer = new byte[1024];
        using var message = new MemoryStream();
        while (true)
        {
            var count = await stream.ReadAsync(buffer, cancellationToken);
            if (count == 0)
                throw new EndOfStreamException("Runtime closed the connection before sending a response.");

            var newline = Array.IndexOf(buffer, (byte)'\n', 0, count);
            var bytesToWrite = newline >= 0 ? newline : count;
            if (message.Length + bytesToWrite > MessageLimitBytes)
                throw new InvalidDataException("Runtime response exceeds the message limit.");
            message.Write(buffer, 0, bytesToWrite);
            if (newline >= 0)
                break;
        }

        var response = Encoding.UTF8.GetString(message.ToArray()).TrimEnd('\r');
        if (string.IsNullOrWhiteSpace(response))
            throw new InvalidDataException("Runtime returned an empty response.");
        return response;
    }

    private static void EnsureSuccess(RuntimeResponse response)
    {
        if (response.Version != 1)
            throw new InvalidDataException($"Unsupported runtime protocol version: {response.Version}.");
        if (!response.Ok)
            throw new RuntimeRejectedException(response.Error ?? "Runtime rejected the request.");
    }

    private sealed class RuntimeResponse
    {
        public int Version { get; set; }
        public bool Ok { get; set; }
        public string? Runtime { get; set; }
        public RuntimeSettings? Settings { get; set; }
        public string? Error { get; set; }
    }
}

public sealed record RuntimeStatus(string Runtime, RuntimeSettings Settings);

public sealed class RuntimeRejectedException(string message) : Exception(message);
