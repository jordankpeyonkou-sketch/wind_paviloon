namespace WvxStudio.Models;

public sealed class RuntimeSettings
{
    public int SchemaVersion { get; set; } = 1;
    public string Profile { get; set; } = "Work";
    public string Theme { get; set; } = "OneUiFrosted";
    public double Blur { get; set; } = 24;
    public double Refraction { get; set; } = 0.35;
    public double Dispersion { get; set; } = 0.04;
    public double TintOpacity { get; set; } = 55;
    public double CornerRadius { get; set; } = 28;
    public List<string> EnabledWidgets { get; set; } = [];

    public RuntimeSettings Copy() => new()
    {
        SchemaVersion = SchemaVersion,
        Profile = Profile,
        Theme = Theme,
        Blur = Blur,
        Refraction = Refraction,
        Dispersion = Dispersion,
        TintOpacity = TintOpacity,
        CornerRadius = CornerRadius,
        EnabledWidgets = [.. EnabledWidgets]
    };

    public void Validate()
    {
        if (SchemaVersion != 1)
            throw new InvalidDataException($"Unsupported settings schema version: {SchemaVersion}.");
        if (string.IsNullOrWhiteSpace(Profile))
            throw new InvalidDataException("Profile cannot be empty.");
        if (string.IsNullOrWhiteSpace(Theme))
            throw new InvalidDataException("Theme cannot be empty.");
        ValidateRange(Blur, 0, 100, nameof(Blur));
        ValidateRange(Refraction, 0, 1, nameof(Refraction));
        ValidateRange(Dispersion, 0, 1, nameof(Dispersion));
        ValidateRange(TintOpacity, 0, 100, nameof(TintOpacity));
        ValidateRange(CornerRadius, 0, 64, nameof(CornerRadius));

        var knownWidgets = new HashSet<string>(StringComparer.Ordinal)
        {
            "clock", "weather", "system-monitor", "media-player", "calendar"
        };
        if (EnabledWidgets is null ||
            EnabledWidgets.Any(widget => !knownWidgets.Contains(widget)) ||
            EnabledWidgets.Distinct(StringComparer.Ordinal).Count() != EnabledWidgets.Count)
            throw new InvalidDataException("Enabled widgets contain an unknown or duplicate ID.");
    }

    private static void ValidateRange(double value, double minimum, double maximum, string name)
    {
        if (!double.IsFinite(value) || value < minimum || value > maximum)
            throw new InvalidDataException($"{name} must be between {minimum} and {maximum}.");
    }
}
