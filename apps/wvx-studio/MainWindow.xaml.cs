using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using WvxStudio.Models;
using WvxStudio.Services;
using Windows.UI;

namespace WvxStudio;

public sealed partial class MainWindow : Window
{
    private readonly RuntimePipeClient _runtime = new();
    private RuntimeSettings _settings = new();
    private bool _initializing;
    private bool _requestInProgress;
    private bool _hasRuntimeSettings;

    private static readonly IReadOnlyDictionary<string, (string Title, string Subtitle)> Pages =
        new Dictionary<string, (string, string)>
        {
            ["Overview"] = ("Overview", "Your desktop, thoughtfully redesigned."),
            ["Widgets"] = ("Widgets", "Choose which widgets are part of your desktop."),
            ["Appearance"] = ("Appearance", "Fine-tune the materials and details."),
            ["Profiles"] = ("Profiles & settings", "A setup for every kind of day.")
        };

    public MainWindow()
    {
        InitializeComponent();
        Title = "WVX Studio";
        ExtendsContentIntoTitleBar = false;
        SetPage("Overview");
        Activated += OnActivated;
    }

    private async void OnActivated(object sender, WindowActivatedEventArgs e)
    {
        Activated -= OnActivated;
        await RefreshAsync();
    }

    private void OnNavigate(object sender, RoutedEventArgs e)
    {
        if (sender is Button { Tag: string page })
            SetPage(page);
    }

    private void SetPage(string name)
    {
        OverviewPage.Visibility = name == "Overview" ? Visibility.Visible : Visibility.Collapsed;
        WidgetsPage.Visibility = name == "Widgets" ? Visibility.Visible : Visibility.Collapsed;
        AppearancePage.Visibility = name == "Appearance" ? Visibility.Visible : Visibility.Collapsed;
        ProfilesPage.Visibility = name == "Profiles" ? Visibility.Visible : Visibility.Collapsed;
        if (!Pages.TryGetValue(name, out var page))
            return;

        PageTitle.Text = page.Title;
        PageSubtitle.Text = page.Subtitle;
        foreach (var (key, button) in new[]
        {
            ("Overview", OverviewNav), ("Widgets", WidgetsNav),
            ("Appearance", AppearanceNav), ("Profiles", ProfilesNav)
        })
            button.Background = key == name
                ? new SolidColorBrush(Color.FromArgb(255, 48, 52, 74))
                : new SolidColorBrush(Colors.Transparent);
    }

    private async void OnRefresh(object sender, RoutedEventArgs e) => await RefreshAsync();

    private async Task RefreshAsync()
    {
        if (_requestInProgress)
            return;

        _requestInProgress = true;
        SetBusy(true, "Reading settings from WVX runtime…");
        try
        {
            var status = await _runtime.GetSettingsAsync();
            _settings = status.Settings;
            _hasRuntimeSettings = true;
            ApplySettingsToControls(_settings);
            SetRuntimeConnected(status.Runtime);
            FooterMessage.Text = "Settings refreshed from the running runtime.";
        }
        catch (RuntimeRejectedException ex)
        {
            SetRuntimeError(ex.Message, hasSettings: false);
            FooterMessage.Text = "Runtime rejected the settings request.";
        }
        catch (Exception ex)
        {
            SetRuntimeFailure(ex);
            FooterMessage.Text = "Runtime settings could not be read. Changes have not been saved.";
        }
        finally
        {
            SetBusy(false);
            _requestInProgress = false;
        }
    }

    private async void OnApply(object sender, RoutedEventArgs e)
    {
        if (_requestInProgress || !_hasRuntimeSettings)
            return;

        try
        {
            var draft = ReadSettingsFromControls();
            draft.Validate();
            _requestInProgress = true;
            SetBusy(true, "Sending settings to WVX runtime…");
            await _runtime.SetSettingsAsync(draft);
            _settings = draft;
            SetRuntimeConnected("running");
            UpdateOverview(draft);
            FooterMessage.Text = "Settings applied to the running runtime.";
        }
        catch (RuntimeRejectedException ex)
        {
            SetRuntimeError(ex.Message);
            FooterMessage.Text = "Runtime rejected these settings; no local copy was saved.";
        }
        catch (Exception ex)
        {
            SetRuntimeFailure(ex);
            FooterMessage.Text = "Settings were not confirmed by the runtime. No local copy was saved.";
        }
        finally
        {
            SetBusy(false);
            _requestInProgress = false;
        }
    }

    private void ApplySettingsToControls(RuntimeSettings settings)
    {
        _initializing = true;
        try
        {
            SetComboByTag(ThemeCombo, settings.Theme);
            SetComboByTag(ProfileCombo, settings.Profile);
            BlurSlider.Value = settings.Blur;
            RefractionSlider.Value = settings.Refraction;
            DispersionSlider.Value = settings.Dispersion;
            TintSlider.Value = settings.TintOpacity;
            CornerSlider.Value = settings.CornerRadius;
            ClockCheck.IsChecked = settings.EnabledWidgets.Contains("clock");
            UpdateSliderLabels();
            UpdateOverview(settings);
        }
        finally
        {
            _initializing = false;
        }
    }

    private RuntimeSettings ReadSettingsFromControls()
    {
        var widgets = new List<string>(_settings.EnabledWidgets);
        if (ClockCheck.IsChecked == true && !widgets.Contains("clock"))
            widgets.Add("clock");
        else if (ClockCheck.IsChecked != true)
            widgets.RemoveAll(id => id == "clock");

        return new RuntimeSettings
        {
            SchemaVersion = 1,
            Profile = SelectedTag(ProfileCombo, _settings.Profile),
            Theme = SelectedTag(ThemeCombo, _settings.Theme),
            Blur = BlurSlider.Value,
            Refraction = RefractionSlider.Value,
            Dispersion = DispersionSlider.Value,
            TintOpacity = TintSlider.Value,
            CornerRadius = CornerSlider.Value,
            EnabledWidgets = widgets
        };
    }

    private static string SelectedTag(ComboBox comboBox, string fallback) =>
        comboBox.SelectedItem is ComboBoxItem { Tag: string tag } ? tag : fallback;

    private static void SetComboByTag(ComboBox comboBox, string tag)
    {
        var match = comboBox.Items
            .OfType<ComboBoxItem>()
            .FirstOrDefault(item => item.Tag is string itemTag && itemTag == tag);
        if (match is not null)
            comboBox.SelectedItem = match;
        else
            comboBox.SelectedIndex = -1;
    }

    private void OnWidgetChanged(object sender, RoutedEventArgs e)
    {
        if (!_initializing)
        {
            var count = _settings.EnabledWidgets.Count(id => id != "clock") + (ClockCheck.IsChecked == true ? 1 : 0);
            WidgetCount.Text = count.ToString();
        }
    }

    private void OnThemeChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!_initializing && ThemeSummary is not null)
            ThemeSummary.Text = ThemeCombo.SelectedItem is ComboBoxItem item ? item.Content?.ToString() ?? "—" : "—";
    }

    private void OnProfileChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!_initializing && ProfileSummary is not null)
            ProfileSummary.Text = ProfileCombo.SelectedItem is ComboBoxItem item ? item.Content?.ToString() ?? "—" : "—";
    }

    private void OnMaterialSliderChanged(object sender, Microsoft.UI.Xaml.Controls.Primitives.RangeBaseValueChangedEventArgs e)
    {
        if (!_initializing && DispersionValue is not null)
            UpdateSliderLabels();
    }

    private void UpdateSliderLabels()
    {
        BlurValue.Text = $"{BlurSlider.Value:0}";
        TintValue.Text = $"{TintSlider.Value:0}%";
        CornerValue.Text = $"{CornerSlider.Value:0}";
        RefractionValue.Text = $"{RefractionSlider.Value:0.00}";
        DispersionValue.Text = $"{DispersionSlider.Value:0.00}";
    }

    private void UpdateOverview(RuntimeSettings settings)
    {
        WidgetCount.Text = settings.EnabledWidgets.Count.ToString();
        ThemeSummary.Text = ThemeDisplayName(settings.Theme);
        ProfileSummary.Text = settings.Profile;
        BlurSummary.Text = $"{settings.Blur:0}";
        TintSummary.Text = $"{settings.TintOpacity:0}%";
    }

    private static string ThemeDisplayName(string theme) => theme switch
    {
        "OneUiFrosted" => "One UI Frosted",
        "LiquidGlass" => "Liquid Glass",
        "LiquidGlassClear" => "Liquid Glass Clear",
        _ => theme
    };

    private void SetBusy(bool busy, string? message = null)
    {
        if (message is not null)
            BannerText.Text = message;
        foreach (var button in new[] { OverviewNav, WidgetsNav, AppearanceNav, ProfilesNav })
            button.IsEnabled = !busy;
        RefreshButton.IsEnabled = !busy;
        ApplyWidgetsButton.IsEnabled = !busy && _hasRuntimeSettings;
        ApplyAppearanceButton.IsEnabled = !busy && _hasRuntimeSettings;
        ApplyProfileButton.IsEnabled = !busy && _hasRuntimeSettings;
    }

    private void SetRuntimeConnected(string runtime)
    {
        _hasRuntimeSettings = true;
        UpdateActionAvailability();
        SetStatusVisual("#78D6A0", "#1D342B", $"Runtime connected · {runtime}", "Connected to the running WVX runtime.");
    }

    private void SetRuntimeFailure(Exception exception)
    {
        _hasRuntimeSettings = false;
        UpdateActionAvailability();
        var (title, color, background, detail) = exception switch
        {
            TimeoutException => ("Runtime timed out", "#E3AC6A", "#352C21", exception.Message),
            EndOfStreamException => ("Runtime protocol error", "#F28A91", "#3A252B", "Runtime closed the connection before replying."),
            InvalidDataException => ("Runtime protocol error", "#F28A91", "#3A252B", exception.Message),
            System.Text.Json.JsonException => ("Runtime protocol error", "#F28A91", "#3A252B", "Runtime returned invalid JSON."),
            IOException => ("Runtime offline or unavailable", "#E3AC6A", "#352C21", "Could not connect to the WVX runtime. Check that it is running."),
            _ => ("Runtime request failed", "#F28A91", "#3A252B", exception.Message)
        };
        SetStatusVisual(color, background, title, detail);
    }

    private void SetRuntimeError(string error, bool hasSettings = true)
    {
        _hasRuntimeSettings = hasSettings;
        UpdateActionAvailability();
        SetStatusVisual("#F28A91", "#3A252B", "Runtime reported an error", error);
    }

    private void UpdateActionAvailability()
    {
        var canApply = _hasRuntimeSettings && !_requestInProgress;
        ApplyWidgetsButton.IsEnabled = canApply;
        ApplyAppearanceButton.IsEnabled = canApply;
        ApplyProfileButton.IsEnabled = canApply;
    }

    private void SetStatusVisual(string color, string background, string title, string detail)
    {
        var brush = new SolidColorBrush(ColorFromHex(color));
        StatusDot.Fill = brush;
        BannerDot.Fill = brush;
        StatusBanner.Background = new SolidColorBrush(ColorFromHex(background));
        SidebarStatusText.Text = title;
        BannerText.Text = $"{title}: {detail}";
        ConnectionDetail.Text = detail;
    }

    private static Color ColorFromHex(string hex) =>
        Color.FromArgb(255,
            Convert.ToByte(hex.Substring(1, 2), 16),
            Convert.ToByte(hex.Substring(3, 2), 16),
            Convert.ToByte(hex.Substring(5, 2), 16));
}
