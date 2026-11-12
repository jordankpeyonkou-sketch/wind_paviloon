use std::{
    error::Error,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use eframe::egui::{
    self, Align, Align2, Button, Color32, Frame, Layout, Margin, RichText, Stroke, Vec2,
};
use serde::{Deserialize, Serialize};

const BACKGROUND: Color32 = Color32::from_rgb(15, 18, 26);
const SIDEBAR: Color32 = Color32::from_rgb(19, 23, 33);
const PANEL: Color32 = Color32::from_rgb(25, 30, 42);
const PANEL_HOVER: Color32 = Color32::from_rgb(34, 41, 56);
const SETTINGS_SCHEMA_VERSION: u32 = 1;
const MAX_SETTINGS_FILE_SIZE: u64 = 1024 * 1024;
static SETTINGS_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
const ACCENT: Color32 = Color32::from_rgb(117, 153, 255);
const TEXT: Color32 = Color32::from_rgb(239, 242, 250);
const MUTED: Color32 = Color32::from_rgb(145, 154, 174);
const GREEN: Color32 = Color32::from_rgb(105, 216, 171);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Page {
    #[default]
    Overview,
    Widgets,
    Appearance,
    Profiles,
    System,
}

impl Page {
    const ALL: [Self; 5] = [
        Self::Overview,
        Self::Widgets,
        Self::Appearance,
        Self::Profiles,
        Self::System,
    ];

    fn title(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Widgets => "Widget gallery",
            Self::Appearance => "Appearance",
            Self::Profiles => "Profiles",
            Self::System => "Windows effects",
        }
    }

    fn subtitle(self) -> &'static str {
        match self {
            Self::Overview => "Your desktop, thoughtfully redesigned.",
            Self::Widgets => "Choose the widgets that make your desktop yours.",
            Self::Appearance => "Fine-tune the materials and details.",
            Self::Profiles => "A setup for every kind of day.",
            Self::System => "Windows-wide styling is not active in this prototype.",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Overview => "⌂",
            Self::Widgets => "▦",
            Self::Appearance => "◐",
            Self::Profiles => "◷",
            Self::System => "▤",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
enum ThemePreset {
    #[default]
    OneUiFrosted,
    LiquidGlass,
    LiquidGlassClear,
    Minimal,
}

impl ThemePreset {
    const ALL: [Self; 4] = [
        Self::OneUiFrosted,
        Self::LiquidGlass,
        Self::LiquidGlassClear,
        Self::Minimal,
    ];

    fn title(self) -> &'static str {
        match self {
            Self::OneUiFrosted => "One UI Frosted",
            Self::LiquidGlass => "Liquid Glass",
            Self::LiquidGlassClear => "Liquid Glass Clear",
            Self::Minimal => "Minimal",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::OneUiFrosted => "Soft, diffused glass with a gentle tint.",
            Self::LiquidGlass => "Prismatic highlights and liquid depth.",
            Self::LiquidGlassClear => "A brighter, clearer glass treatment.",
            Self::Minimal => "Quiet surfaces with almost no effects.",
        }
    }

    fn accent(self) -> Color32 {
        match self {
            Self::OneUiFrosted => Color32::from_rgb(117, 153, 255),
            Self::LiquidGlass => Color32::from_rgb(185, 140, 255),
            Self::LiquidGlassClear => Color32::from_rgb(99, 211, 221),
            Self::Minimal => Color32::from_rgb(174, 185, 204),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
enum Profile {
    #[default]
    Work,
    Gaming,
    Minimal,
    PowerSaving,
}

impl Profile {
    const ALL: [Self; 4] = [Self::Work, Self::Gaming, Self::Minimal, Self::PowerSaving];

    fn title(self) -> &'static str {
        match self {
            Self::Work => "Work",
            Self::Gaming => "Gaming",
            Self::Minimal => "Minimal",
            Self::PowerSaving => "Power saving",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Work => "A balanced setup for everyday focus.",
            Self::Gaming => "A clean desktop with restrained effects.",
            Self::Minimal => "Only the essentials, with quiet surfaces.",
            Self::PowerSaving => "Keep animations and visual effects low.",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Work => "⌘",
            Self::Gaming => "◇",
            Self::Minimal => "○",
            Self::PowerSaving => "ϟ",
        }
    }

    fn settings(self) -> Settings {
        let mut settings = Settings {
            profile: self,
            ..Settings::default()
        };
        match self {
            Self::Work => {}
            Self::Gaming => {
                settings.theme = ThemePreset::LiquidGlassClear;
                settings.blur = 12.0;
                settings.refraction = 0.18;
                settings.enabled_widgets = vec!["system-monitor".into(), "media-player".into()];
            }
            Self::Minimal => {
                settings.theme = ThemePreset::Minimal;
                settings.blur = 4.0;
                settings.refraction = 0.0;
                settings.dispersion = 0.0;
                settings.enabled_widgets = vec!["clock".into()];
            }
            Self::PowerSaving => {
                settings.theme = ThemePreset::OneUiFrosted;
                settings.blur = 8.0;
                settings.refraction = 0.0;
                settings.dispersion = 0.0;
                settings.enabled_widgets = vec!["clock".into(), "calendar".into()];
            }
        }
        settings
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Settings {
    profile: Profile,
    theme: ThemePreset,
    blur: f32,
    refraction: f32,
    dispersion: f32,
    tint_opacity: f32,
    corner_radius: f32,
    enabled_widgets: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct SettingsDocument {
    schema_version: u32,
    settings: Settings,
}

impl Settings {
    fn validate(&self) -> io::Result<()> {
        for (name, value, min, max) in [
            ("blur", self.blur, 0.0, 48.0),
            ("refraction", self.refraction, 0.0, 1.0),
            ("dispersion", self.dispersion, 0.0, 0.12),
            ("tint_opacity", self.tint_opacity, 0.0, 100.0),
            ("corner_radius", self.corner_radius, 8.0, 44.0),
        ] {
            if !value.is_finite() || !(min..=max).contains(&value) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{name} must be a finite value between {min} and {max}."),
                ));
            }
        }

        for (index, id) in self.enabled_widgets.iter().enumerate() {
            if !WIDGETS.iter().any(|widget| widget.id == id) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Unknown widget id: {id}."),
                ));
            }
            if self.enabled_widgets[..index].contains(id) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Duplicate widget id: {id}."),
                ));
            }
        }

        Ok(())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            profile: Profile::Work,
            theme: ThemePreset::OneUiFrosted,
            blur: 24.0,
            refraction: 0.35,
            dispersion: 0.04,
            tint_opacity: 55.0,
            corner_radius: 28.0,
            enabled_widgets: vec!["clock".into(), "calendar".into()],
        }
    }
}

struct WidgetInfo {
    id: &'static str,
    icon: &'static str,
    name: &'static str,
    description: &'static str,
    category: &'static str,
}

const WIDGETS: [WidgetInfo; 5] = [
    WidgetInfo {
        id: "clock",
        icon: "◷",
        name: "Adaptive clock",
        description: "A clean clock, ready to adapt to your wallpaper.",
        category: "TIME",
    },
    WidgetInfo {
        id: "weather",
        icon: "☼",
        name: "Weather",
        description: "A simple forecast card. Live weather is not connected.",
        category: "AT A GLANCE",
    },
    WidgetInfo {
        id: "system-monitor",
        icon: "⌁",
        name: "System monitor",
        description: "A desktop card for system stats. Data is not connected.",
        category: "SYSTEM",
    },
    WidgetInfo {
        id: "media-player",
        icon: "♫",
        name: "Media player",
        description: "A compact player surface. Playback is not connected.",
        category: "MEDIA",
    },
    WidgetInfo {
        id: "calendar",
        icon: "▦",
        name: "Calendar",
        description: "A minimal calendar for the days ahead.",
        category: "AT A GLANCE",
    },
];

struct GlassDeskStudio {
    page: Page,
    settings: Settings,
    status: String,
    error: Option<String>,
}

impl GlassDeskStudio {
    fn new(context: &eframe::CreationContext<'_>) -> Self {
        configure_visuals(&context.egui_ctx);
        let (settings, error) = load_settings();
        let status = if error.is_none() {
            "Your settings are saved on this device.".to_owned()
        } else {
            "Could not read settings; showing the defaults.".to_owned()
        };
        Self {
            page: Page::Overview,
            settings,
            status,
            error,
        }
    }

    fn save_settings(&mut self) {
        match settings_path().and_then(|path| write_settings_file(&path, &self.settings)) {
            Ok(()) => {
                self.status = "Changes saved on this device.".into();
                self.error = None;
            }
            Err(error) => {
                self.status = "Could not save your settings.".into();
                self.error = Some(error.to_string());
            }
        }
    }

    fn sidebar(&mut self, context: &egui::Context) {
        egui::SidePanel::left("studio_navigation")
            .exact_width(232.0)
            .resizable(false)
            .frame(
                Frame::none()
                    .fill(SIDEBAR)
                    .inner_margin(Margin::symmetric(16.0, 20.0)),
            )
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("◈").size(23.0).color(ACCENT));
                    ui.label(RichText::new("GlassDesk").size(19.0).strong().color(TEXT));
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new("DESKTOP STUDIO")
                        .size(10.0)
                        .strong()
                        .color(MUTED),
                );
                ui.add_space(30.0);

                for page in Page::ALL {
                    let selected = self.page == page;
                    let button = Button::new(
                        RichText::new(format!("{}    {}", page.icon(), page.title()))
                            .size(14.0)
                            .color(if selected { TEXT } else { MUTED }),
                    )
                    .fill(if selected {
                        PANEL_HOVER
                    } else {
                        Color32::TRANSPARENT
                    })
                    .stroke(if selected {
                        Stroke::new(1.0_f32, Color32::from_rgb(49, 61, 85))
                    } else {
                        Stroke::NONE
                    })
                    .rounding(10.0)
                    .min_size(Vec2::new(196.0, 42.0));
                    if ui.add(button).clicked() {
                        self.page = page;
                    }
                    ui.add_space(4.0);
                }

                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    ui.label(RichText::new("Prototype · Phase 0").size(11.0).color(MUTED));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.colored_label(GREEN, "●");
                        ui.label(RichText::new("Running locally").size(12.0).color(TEXT));
                    });
                    ui.separator();
                    ui.label(RichText::new("GlassDesk Studio").size(12.0).color(TEXT));
                });
            });
    }

    fn top_bar(&self, context: &egui::Context) {
        egui::TopBottomPanel::top("studio_header")
            .exact_height(94.0)
            .frame(
                Frame::none()
                    .fill(BACKGROUND)
                    .inner_margin(Margin::symmetric(30.0, 19.0)),
            )
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading(
                            RichText::new(self.page.title())
                                .size(24.0)
                                .strong()
                                .color(TEXT),
                        );
                        ui.label(RichText::new(self.page.subtitle()).size(13.0).color(MUTED));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new("LOCAL PREVIEW")
                                .size(10.0)
                                .strong()
                                .color(MUTED),
                        );
                        ui.colored_label(GREEN, "●");
                    });
                });
            });
    }

    fn status_bar(&self, context: &egui::Context) {
        egui::TopBottomPanel::bottom("studio_status")
            .exact_height(30.0)
            .frame(
                Frame::none()
                    .fill(SIDEBAR)
                    .inner_margin(Margin::symmetric(16.0, 5.0)),
            )
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    if let Some(error) = &self.error {
                        ui.colored_label(
                            Color32::from_rgb(255, 157, 157),
                            format!("Settings issue: {error}"),
                        );
                    } else {
                        ui.colored_label(GREEN, "●");
                        ui.label(RichText::new(&self.status).size(10.0).color(MUTED));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new("%APPDATA%  ·  GlassDesk  ·  Local only")
                                .size(10.0)
                                .color(MUTED),
                        );
                    });
                });
            });
    }

    fn overview(&mut self, ui: &mut egui::Ui) -> bool {
        let active_count = self.settings.enabled_widgets.len();
        let active_widgets = WIDGETS
            .iter()
            .filter(|widget| {
                self.settings
                    .enabled_widgets
                    .iter()
                    .any(|enabled| enabled == widget.id)
            })
            .map(|widget| widget.name)
            .collect::<Vec<_>>()
            .join(" · ");
        section_label(ui, "YOUR DESKTOP");
        ui.add_space(10.0);

        ui.columns(3, |columns| {
            info_card(
                &mut columns[0],
                "ACTIVE WIDGETS",
                &format!("{active_count:02}"),
                &active_widgets,
                "▦",
            );
            info_card(
                &mut columns[1],
                "CURRENT THEME",
                self.settings.theme.title(),
                "Personalized for your desktop",
                "◐",
            );
            info_card(
                &mut columns[2],
                "PROFILE",
                self.settings.profile.title(),
                "Ready when you are",
                "◷",
            );
        });

        ui.add_space(24.0);
        ui.columns(2, |columns| {
            Frame::group(columns[0].style())
                .fill(PANEL)
                .rounding(14.0)
                .inner_margin(Margin::same(18.0))
                .show(&mut columns[0], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("A little more you").size(17.0).strong().color(TEXT));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new("THEME PREVIEW").size(9.0).strong().color(MUTED));
                        });
                    });
                    ui.add_space(12.0);
                    preview_card(ui, &self.settings);
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new("Material preview only — the desktop backdrop is not captured or blurred yet.")
                            .size(11.0)
                            .color(MUTED),
                    );
                    ui.add_space(12.0);
                    if ui
                        .add(
                            Button::new(RichText::new("Customize appearance  →").color(TEXT))
                                .fill(PANEL_HOVER)
                                .rounding(8.0),
                        )
                        .clicked()
                    {
                        self.page = Page::Appearance;
                    }
                });

            Frame::group(columns[1].style())
                .fill(PANEL)
                .rounding(14.0)
                .inner_margin(Margin::same(18.0))
                .show(&mut columns[1], |ui| {
                    ui.label(RichText::new("Your widgets").size(17.0).strong().color(TEXT));
                    ui.add_space(8.0);
                    for widget in WIDGETS.iter().filter(|widget| {
                        self.settings
                            .enabled_widgets
                            .iter()
                            .any(|enabled| enabled == widget.id)
                    }) {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(widget.icon).size(18.0).color(ACCENT));
                            ui.label(RichText::new(widget.name).size(13.0).color(TEXT));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(RichText::new("ACTIVE").size(9.0).strong().color(GREEN));
                            });
                        });
                        ui.add_space(9.0);
                    }
                    ui.add_space(5.0);
                    if ui
                        .add(
                            Button::new(RichText::new("Browse widget gallery  →").color(TEXT))
                                .fill(PANEL_HOVER)
                                .rounding(8.0),
                        )
                        .clicked()
                    {
                        self.page = Page::Widgets;
                    }
                });
        });

        ui.add_space(16.0);
        false
    }

    fn widget_gallery(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.label(
            RichText::new("A FEW PLACES TO START")
                .size(10.0)
                .strong()
                .color(MUTED),
        );
        ui.add_space(14.0);

        for row in WIDGETS.chunks(2) {
            ui.columns(2, |columns| {
                for (column, widget) in columns.iter_mut().zip(row) {
                    let active = self
                        .settings
                        .enabled_widgets
                        .iter()
                        .any(|id| id == widget.id);
                    Frame::group(column.style())
                        .fill(PANEL)
                        .rounding(14.0)
                        .inner_margin(Margin::same(17.0))
                        .show(column, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(widget.icon).size(25.0).color(ACCENT));
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(widget.name).size(15.0).strong().color(TEXT),
                                    );
                                    ui.label(
                                        RichText::new(widget.category)
                                            .size(9.0)
                                            .strong()
                                            .color(MUTED),
                                    );
                                });
                            });
                            ui.add_space(10.0);
                            ui.label(RichText::new(widget.description).size(11.0).color(MUTED));
                            ui.add_space(14.0);
                            let label = if active { "Added" } else { "Add widget" };
                            let button =
                                Button::new(RichText::new(label).size(11.0).color(if active {
                                    GREEN
                                } else {
                                    TEXT
                                }))
                                .fill(if active { PANEL_HOVER } else { ACCENT })
                                .rounding(7.0)
                                .min_size(Vec2::new(100.0, 29.0));
                            if ui.add(button).clicked() {
                                changed = true;
                                if active {
                                    self.settings.enabled_widgets.retain(|id| id != widget.id);
                                } else {
                                    self.settings.enabled_widgets.push(widget.id.to_owned());
                                }
                            }
                        });
                }
            });
            ui.add_space(12.0);
        }
        ui.add_space(6.0);
        ui.label(
            RichText::new("Gallery entries are local prototypes. Live weather, media, and system data are not connected.")
                .size(11.0)
                .color(MUTED),
        );
        changed
    }

    fn appearance(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.columns(2, |columns| {
            let controls = &mut columns[0];
            section_label(controls, "MATERIAL PRESETS");
            controls.add_space(9.0);
            for theme in ThemePreset::ALL {
                let selected = self.settings.theme == theme;
                let response = controls.add(
                    Button::new(
                        RichText::new(format!("{}   {}", if selected { "●" } else { "○" }, theme.title()))
                            .size(13.0)
                            .color(if selected { TEXT } else { MUTED }),
                    )
                    .fill(if selected { PANEL_HOVER } else { PANEL })
                    .stroke(Stroke::new(
                        1.0_f32,
                        if selected { theme.accent() } else { Color32::TRANSPARENT },
                    ))
                    .rounding(9.0)
                    .min_size(Vec2::new(310.0, 42.0)),
                );
                if response.clicked() {
                    self.settings.theme = theme;
                    changed = true;
                }
                controls.add_space(5.0);
            }

            controls.add_space(19.0);
            section_label(controls, "MATERIAL CONTROLS");
            controls.add_space(8.0);
            changed |= slider(controls, "Blur radius", &mut self.settings.blur, 0.0..=48.0, "px");
            changed |= slider(
                controls,
                "Glass tint",
                &mut self.settings.tint_opacity,
                0.0..=100.0,
                "%",
            );
            changed |= slider(
                controls,
                "Corner radius",
                &mut self.settings.corner_radius,
                8.0..=44.0,
                "px",
            );
            changed |= slider(
                controls,
                "Refraction",
                &mut self.settings.refraction,
                0.0..=1.0,
                "",
            );
            changed |= slider(
                controls,
                "Dispersion",
                &mut self.settings.dispersion,
                0.0..=0.12,
                "",
            );

            Frame::group(columns[1].style())
                .fill(PANEL)
                .rounding(14.0)
                .inner_margin(Margin::same(18.0))
                .show(&mut columns[1], |ui| {
                    ui.label(RichText::new("Live preview").size(16.0).strong().color(TEXT));
                    ui.add_space(12.0);
                    preview_card(ui, &self.settings);
                    ui.add_space(14.0);
                    ui.label(
                        RichText::new(self.settings.theme.description())
                            .size(12.0)
                            .color(MUTED),
                    );
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("Controls preview a material only. Desktop capture and shader effects are not enabled yet.")
                            .size(11.0)
                            .color(MUTED),
                    );
                });
        });
        if changed {
            self.settings.profile = Profile::Work;
        }
        changed
    }

    fn profiles(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.label(
            RichText::new("PICK UP WHERE YOU LEFT OFF")
                .size(10.0)
                .strong()
                .color(MUTED),
        );
        ui.add_space(15.0);
        ui.columns(2, |columns| {
            for (column, profile) in columns.iter_mut().zip(Profile::ALL) {
                let selected = self.settings.profile == profile;
                Frame::group(column.style())
                    .fill(PANEL)
                    .rounding(14.0)
                    .inner_margin(Margin::same(18.0))
                    .show(column, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(profile.icon()).size(23.0).color(ACCENT));
                            ui.label(
                                RichText::new(profile.title())
                                    .size(16.0)
                                    .strong()
                                    .color(TEXT),
                            );
                        });
                        ui.add_space(8.0);
                        ui.label(RichText::new(profile.description()).size(11.0).color(MUTED));
                        ui.add_space(13.0);
                        let label = if selected {
                            "Current profile"
                        } else {
                            "Use this profile"
                        };
                        if ui
                            .add(
                                Button::new(RichText::new(label).color(TEXT))
                                    .fill(if selected { PANEL_HOVER } else { ACCENT })
                                    .rounding(7.0),
                            )
                            .clicked()
                            && !selected
                        {
                            self.settings = profile.settings();
                            changed = true;
                        }
                    });
                column.add_space(12.0);
            }
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new("Profiles change local widget selections and material preview settings. They do not alter Windows power or gaming settings.")
                .size(11.0)
                .color(MUTED),
        );
        changed
    }

    fn system_page(&self, ui: &mut egui::Ui) {
        Frame::group(ui.style())
            .fill(PANEL)
            .rounding(14.0)
            .inner_margin(Margin::same(20.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(ACCENT, "◈");
                    ui.label(RichText::new("Shell effects are not enabled").size(16.0).strong().color(TEXT));
                });
                ui.add_space(10.0);
                ui.label(
                    RichText::new("GlassDesk does not currently change Windows windows, Explorer, or the taskbar. These integrations require additional system-specific work and remain off in this prototype.")
                        .size(12.0)
                        .color(MUTED),
                );
                ui.add_space(18.0);
                for (name, detail) in [
                    ("Window materials", "DWM integration · planned"),
                    ("Taskbar styling", "Optional shell integration · planned"),
                    ("Desktop glass underlay", "Window tracking · planned"),
                    ("Wallpaper analysis", "Adaptive clock · planned"),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("○").size(15.0).color(MUTED));
                        ui.vertical(|ui| {
                            ui.label(RichText::new(name).size(12.0).color(TEXT));
                            ui.label(RichText::new(detail).size(10.0).color(MUTED));
                        });
                    });
                    ui.add_space(10.0);
                }
            });
    }
}

impl eframe::App for GlassDeskStudio {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.sidebar(context);
        self.top_bar(context);
        self.status_bar(context);

        let mut changed = false;
        egui::CentralPanel::default()
            .frame(
                Frame::none()
                    .fill(BACKGROUND)
                    .inner_margin(Margin::symmetric(30.0, 6.0)),
            )
            .show(context, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        changed = match self.page {
                            Page::Overview => self.overview(ui),
                            Page::Widgets => self.widget_gallery(ui),
                            Page::Appearance => self.appearance(ui),
                            Page::Profiles => self.profiles(ui),
                            Page::System => {
                                self.system_page(ui);
                                false
                            }
                        };
                    });
            });

        if changed {
            self.save_settings();
        }
    }
}

pub fn run() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("GlassDesk Studio")
            .with_inner_size([1180.0, 780.0])
            .with_min_inner_size([820.0, 620.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "GlassDesk Studio",
        options,
        Box::new(|context| Box::new(GlassDeskStudio::new(context))),
    )
}

fn configure_visuals(context: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BACKGROUND;
    visuals.window_fill = PANEL;
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = ACCENT;
    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.inactive.bg_fill = PANEL;
    visuals.widgets.hovered.bg_fill = PANEL_HOVER;
    visuals.widgets.active.bg_fill = ACCENT;
    context.set_visuals(visuals);
}

fn settings_path() -> Result<PathBuf, Box<dyn Error>> {
    let app_data = std::env::var_os("APPDATA")
        .ok_or("The Windows APPDATA directory is unavailable; settings cannot be saved.")?;
    Ok(PathBuf::from(app_data)
        .join("GlassDesk")
        .join("settings.json"))
}

fn load_settings() -> (Settings, Option<String>) {
    let path = match settings_path() {
        Ok(path) => path,
        Err(error) => return (Settings::default(), Some(error.to_string())),
    };
    if !path.exists() {
        return (Settings::default(), None);
    }
    let result = read_settings_file(&path);
    match result {
        Ok(settings) => (settings, None),
        Err(error) => (
            Settings::default(),
            Some(format!("{}: {error}", path.display())),
        ),
    }
}

fn read_settings_file(path: &Path) -> Result<Settings, Box<dyn Error>> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_SETTINGS_FILE_SIZE + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_SETTINGS_FILE_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Settings file exceeds the 1 MiB size limit.",
        )
        .into());
    }

    Ok(decode_settings(&bytes)?)
}

fn decode_settings(bytes: &[u8]) -> Result<Settings, Box<dyn Error>> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    let settings = if value
        .as_object()
        .is_some_and(|object| object.contains_key("schema_version"))
    {
        let document: SettingsDocument = serde_json::from_value(value)?;
        if document.schema_version != SETTINGS_SCHEMA_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Unsupported settings schema version: {}.",
                    document.schema_version
                ),
            )
            .into());
        }
        document.settings
    } else {
        serde_json::from_value(value)?
    };
    settings.validate()?;
    Ok(settings)
}

fn write_settings_file(path: &Path, settings: &Settings) -> Result<(), Box<dyn Error>> {
    settings.validate()?;
    let contents = serde_json::to_vec_pretty(&SettingsDocument {
        schema_version: SETTINGS_SCHEMA_VERSION,
        settings: settings.clone(),
    })?;
    atomic_write(path, &contents)?;
    Ok(())
}

fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let (temporary_path, mut file) = (0..16)
        .find_map(|_| {
            let counter = SETTINGS_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let temporary_path =
                parent.join(format!(".settings-{}-{counter}.tmp", std::process::id()));
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary_path)
            {
                Ok(file) => Some(Ok((temporary_path, file))),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => None,
                Err(error) => Some(Err(error)),
            }
        })
        .transpose()?
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Could not create a unique temporary settings file.",
            )
        })?;

    let result = (|| {
        file.write_all(contents)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary_path, path)
    })();

    if let Err(error) = result {
        match fs::remove_file(&temporary_path) {
            Ok(()) => Err(error),
            Err(cleanup_error) if cleanup_error.kind() == io::ErrorKind::NotFound => Err(error),
            Err(cleanup_error) => Err(io::Error::new(
                error.kind(),
                format!("{error}; could not remove temporary file: {cleanup_error}"),
            )),
        }
    } else {
        Ok(())
    }
}

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(10.0).strong().color(MUTED));
}

fn info_card(ui: &mut egui::Ui, label: &str, value: &str, detail: &str, icon: &str) {
    Frame::group(ui.style())
        .fill(PANEL)
        .rounding(12.0)
        .inner_margin(Margin::same(15.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).size(18.0).color(ACCENT));
                ui.label(RichText::new(label).size(9.0).strong().color(MUTED));
            });
            ui.add_space(7.0);
            ui.label(RichText::new(value).size(17.0).strong().color(TEXT));
            ui.label(RichText::new(detail).size(10.0).color(MUTED));
        });
}

fn preview_card(ui: &mut egui::Ui, settings: &Settings) {
    let accent = settings.theme.accent();
    let response =
        ui.allocate_response(Vec2::new(ui.available_width(), 170.0), egui::Sense::hover());
    let rect = response.rect;
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, settings.corner_radius, Color32::from_rgb(36, 47, 70));
    painter.circle_filled(
        rect.right_top() + Vec2::new(-35.0, 35.0),
        49.0,
        Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 50),
    );
    painter.rect_filled(
        rect.shrink(8.0),
        (settings.corner_radius - 7.0).max(4.0),
        Color32::from_rgba_unmultiplied(171, 187, 221, (settings.tint_opacity * 0.45) as u8),
    );
    painter.rect_stroke(
        rect.shrink(8.0),
        (settings.corner_radius - 7.0).max(4.0),
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(228, 237, 255, 115)),
    );
    painter.text(
        rect.center() + Vec2::new(0.0, -5.0),
        Align2::CENTER_CENTER,
        "09:41",
        egui::FontId::proportional(46.0),
        TEXT,
    );
    painter.text(
        rect.center() + Vec2::new(0.0, 37.0),
        Align2::CENTER_CENTER,
        "MONDAY  ·  YOUR DESKTOP",
        egui::FontId::proportional(9.0),
        Color32::from_rgb(207, 217, 240),
    );
}

fn slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    unit: &str,
) -> bool {
    let before = *value;
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.0).color(TEXT));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let value_text = if unit.is_empty() {
                format!("{:.2}", *value)
            } else {
                format!("{:.0} {unit}", *value)
            };
            ui.label(RichText::new(value_text).size(10.0).color(MUTED));
        });
    });
    ui.add(egui::Slider::new(value, range).show_value(false))
        .changed()
        || before != *value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_keep_only_known_widget_ids() {
        for profile in Profile::ALL {
            let settings = profile.settings();
            assert!(settings
                .enabled_widgets
                .iter()
                .all(|id| { WIDGETS.iter().any(|widget| widget.id == id) }));
        }
    }

    #[test]
    fn settings_round_trip_through_json() {
        let settings = Settings::default();
        let encoded = serde_json::to_vec(&SettingsDocument {
            schema_version: SETTINGS_SCHEMA_VERSION,
            settings: settings.clone(),
        })
        .expect("settings should serialize");
        let decoded = decode_settings(&encoded).expect("settings should deserialize");

        assert_eq!(decoded, settings);
    }

    #[test]
    fn legacy_settings_are_read_and_validated() {
        let settings = Settings::default();
        let encoded = serde_json::to_vec(&settings).expect("legacy settings should serialize");
        let decoded = decode_settings(&encoded).expect("legacy settings should deserialize");

        assert_eq!(decoded, settings);
    }

    #[test]
    fn settings_reject_invalid_values_and_unknown_widgets() {
        let mut settings = Settings::default();
        settings.blur = f32::NAN;
        assert!(settings.validate().is_err());

        let mut settings = Settings::default();
        settings.enabled_widgets.push("unknown".into());
        assert!(settings.validate().is_err());
    }

    #[test]
    fn settings_reject_unsupported_schema_versions() {
        let encoded = serde_json::json!({
            "schema_version": SETTINGS_SCHEMA_VERSION + 1,
            "settings": Settings::default(),
        });

        assert!(decode_settings(&encoded.to_string().into_bytes()).is_err());
    }

    #[test]
    fn atomic_write_replaces_existing_settings_file() {
        let counter = SETTINGS_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "glassdesk-settings-test-{}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).expect("test directory should be created");
        let path = directory.join("settings.json");
        fs::write(&path, b"old settings").expect("old settings should be written");

        atomic_write(&path, b"new settings").expect("settings should be atomically replaced");

        assert_eq!(
            fs::read(&path).expect("settings should be readable"),
            b"new settings"
        );
        assert_eq!(
            fs::read_dir(&directory)
                .expect("test directory should be readable")
                .count(),
            1,
            "temporary files should be removed after a successful save"
        );
        fs::remove_dir_all(&directory).expect("test directory should be removed");
    }
}
