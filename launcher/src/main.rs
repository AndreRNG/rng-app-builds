#![cfg_attr(windows, windows_subsystem = "windows")]
//! RNG Studio: one window that installs, opens and updates RNG Media's apps.

mod store;

use egui::{Color32, CornerRadius, FontId, Margin, RichText, Stroke, TextureHandle, Vec2};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use store::{About, Installed, Manifest, Release, State};

pub const ICON_BIG: &[u8] = include_bytes!("../assets/studio.png");

const ACCENT: Color32 = Color32::from_rgb(0xff, 0x45, 0x3a);
const GOLD: Color32 = Color32::from_rgb(0xe8, 0xb8, 0x5c);
const TEXT: Color32 = Color32::from_rgb(0xf2, 0xf2, 0xf7);
const DIM: Color32 = Color32::from_rgb(0x98, 0x98, 0x9f);
const BG_TOP: Color32 = Color32::from_rgb(0x26, 0x26, 0x2a);
const BG_BOTTOM: Color32 = Color32::from_rgb(0x12, 0x12, 0x14);

fn builtin_icon(id: &str) -> Option<&'static [u8]> {
    Some(match id {
        "cutcat" => include_bytes!("../assets/cutcat.png"),
        "photofalcon" => include_bytes!("../assets/photofalcon.png"),
        "pdfrog" => include_bytes!("../assets/pdfrog.png"),
        "docdog" => include_bytes!("../assets/docdog.png"),
        "deckduck" => include_bytes!("../assets/deckduck.png"),
        "studio" => include_bytes!("../assets/studio.png"),
        _ => return None,
    })
}

#[derive(Clone)]
enum Job {
    Busy { label: String, frac: Option<f32> },
    Failed(String),
}

#[derive(Default)]
struct Shared {
    manifest: Option<Manifest>,
    loading: bool,
    note: Option<String>,
    state: State,
    jobs: HashMap<String, Job>,
    queue_busy: bool,
}

type Decoded = Arc<Mutex<Vec<(String, Option<egui::ColorImage>)>>>;

enum Shot {
    Loading,
    Ready(TextureHandle),
    Failed,
}

struct App {
    shared: Arc<Mutex<Shared>>,
    icons: HashMap<String, TextureHandle>,
    show_about: bool,
    /// The app whose detail view is open.
    detail: Option<String>,
    /// Screenshot shown full-window.
    zoom: Option<String>,
    shots: HashMap<String, Shot>,
    /// Decoded screenshots from the loader threads, turned into textures on the UI thread.
    decoded: Decoded,
    /// Testing: RNG_STUDIO_SHOT=<png> saves the window once it has settled, then closes it.
    shot_to: Option<(std::path::PathBuf, u32)>,
    zoom_first: bool,
    scroll_to: Option<f32>,
}

const THIRD_PARTY: &str = include_str!("../assets/THIRD-PARTY.txt");

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        style(&cc.egui_ctx);
        let state = store::load_state();
        for (id, i) in &state.apps {
            store::clean_stale(id, &i.dir);
        }
        let app = Self {
            shared: Arc::new(Mutex::new(Shared { state, ..Default::default() })),
            icons: HashMap::new(),
            show_about: false,
            detail: None,
            zoom: None,
            shots: HashMap::new(),
            decoded: Arc::default(),
            shot_to: std::env::var_os("RNG_STUDIO_SHOT").map(|p| (p.into(), 0)),
            zoom_first: false,
            // Testing: RNG_STUDIO_SCROLL=<px> starts the detail view scrolled down.
            scroll_to: std::env::var("RNG_STUDIO_SCROLL").ok().and_then(|v| v.parse().ok()),
        };
        // Testing: RNG_STUDIO_SHOW=<app id>[:zoom] starts on that app's detail view.
        let mut app = app;
        if let Ok(v) = std::env::var("RNG_STUDIO_SHOW") {
            let (id, zoom) = v.split_once(':').map(|(a, b)| (a.to_string(), b == "zoom")).unwrap_or((v.clone(), false));
            app.detail = Some(id);
            app.zoom_first = zoom;
        }
        app.refresh(&cc.egui_ctx);
        app
    }

    fn refresh(&self, ctx: &egui::Context) {
        let shared = self.shared.clone();
        let ctx = ctx.clone();
        shared.lock().unwrap().loading = true;
        std::thread::spawn(move || {
            let r = store::fetch_manifest();
            let mut urls = Vec::new();
            {
                let mut s = shared.lock().unwrap();
                s.loading = false;
                match r {
                    Ok((m, note)) => {
                        store::shortcuts::migrate(&s.state.apps, &m);
                        urls = m.apps.iter().filter_map(|a| a.about.as_ref()).flat_map(|a| a.screenshots.iter().map(|x| x.url.clone())).collect();
                        s.manifest = Some(m);
                        s.note = note;
                    }
                    Err(e) => s.note = Some(e),
                }
            }
            ctx.request_repaint();
            // Fill the screenshot cache in the background, so detail views open with pictures.
            for u in urls {
                let _ = store::screenshot(&u);
            }
        });
    }

    /// Installs/updates the given apps one after another on a worker thread.
    fn run_queue(&self, ctx: &egui::Context, apps: Vec<Release>) {
        let shared = self.shared.clone();
        let ctx = ctx.clone();
        {
            let mut s = shared.lock().unwrap();
            if s.queue_busy {
                return;
            }
            s.queue_busy = true;
            for a in &apps {
                s.jobs.insert(a.id.clone(), Job::Busy { label: "Waiting…".into(), frac: None });
            }
        }
        std::thread::spawn(move || {
            for app in apps {
                let id = app.id.clone();
                let set = |job: Option<Job>| {
                    let mut s = shared.lock().unwrap();
                    match job {
                        Some(j) => s.jobs.insert(id.clone(), j),
                        None => s.jobs.remove(&id),
                    };
                    ctx.request_repaint();
                };
                set(Some(Job::Busy { label: "Starting download…".into(), frac: Some(0.0) }));
                let progress = |done: u64, total: u64| {
                    let frac = if total > 0 { done as f32 / total as f32 } else { 0.0 };
                    let label = if frac >= 0.999 { "Installing…".into() } else { format!("Downloading {} of {}", mb(done), mb(total)) };
                    set(Some(Job::Busy { label, frac: Some(frac) }));
                };
                match store::install(&app, &progress) {
                    Ok(installed) => {
                        shared.lock().unwrap().state.apps.insert(app.id.clone(), installed);
                        set(None);
                    }
                    Err(e) => set(Some(Job::Failed(e))),
                }
            }
            shared.lock().unwrap().queue_busy = false;
            ctx.request_repaint();
        });
    }

    fn self_update(&self, ctx: &egui::Context, rel: Release) {
        let shared = self.shared.clone();
        let ctx = ctx.clone();
        shared.lock().unwrap().jobs.insert("studio".into(), Job::Busy { label: "Updating RNG Studio…".into(), frac: Some(0.0) });
        std::thread::spawn(move || {
            let progress = |done: u64, total: u64| {
                let f = if total > 0 { done as f32 / total as f32 } else { 0.0 };
                shared.lock().unwrap().jobs.insert("studio".into(), Job::Busy { label: format!("Updating RNG Studio… {}", mb(done)), frac: Some(f) });
                ctx.request_repaint();
            };
            if let Err(e) = store::self_update(&rel, &progress) {
                shared.lock().unwrap().jobs.insert("studio".into(), Job::Failed(format!("RNG Studio update failed: {e}")));
                ctx.request_repaint();
            }
        });
    }

    /// Starts loading a screenshot (cache or network) unless it's loaded or on its way.
    fn request_shot(&mut self, ctx: &egui::Context, url: &str) {
        if self.shots.contains_key(url) {
            return;
        }
        self.shots.insert(url.into(), Shot::Loading);
        let (url, out, ctx) = (url.to_string(), self.decoded.clone(), ctx.clone());
        std::thread::spawn(move || {
            let img = store::screenshot(&url).ok().and_then(|b| image::load_from_memory(&b).ok()).map(|i| {
                let i = i.to_rgba8();
                egui::ColorImage::from_rgba_unmultiplied([i.width() as usize, i.height() as usize], i.as_raw())
            });
            out.lock().unwrap().push((url, img));
            ctx.request_repaint();
        });
    }

    fn take_decoded(&mut self, ctx: &egui::Context) {
        let done: Vec<_> = std::mem::take(&mut *self.decoded.lock().unwrap());
        for (url, img) in done {
            let shot = match img {
                Some(ci) => Shot::Ready(ctx.load_texture(&url, ci, egui::TextureOptions::LINEAR.with_mipmap_mode(Some(egui::TextureFilter::Linear)))),
                None => Shot::Failed,
            };
            self.shots.insert(url, shot);
        }
    }

    fn icon(&mut self, ctx: &egui::Context, id: &str) -> Option<TextureHandle> {
        if let Some(t) = self.icons.get(id) {
            return Some(t.clone());
        }
        let img = image::load_from_memory(builtin_icon(id)?).ok()?.to_rgba8();
        let ci = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
        let t = ctx.load_texture(id, ci, egui::TextureOptions::LINEAR);
        self.icons.insert(id.into(), t.clone());
        Some(t)
    }
}

fn mb(b: u64) -> String {
    format!("{:.0} MB", b as f64 / 1_048_576.0)
}

fn style(ctx: &egui::Context) {
    ctx.set_visuals(egui::Visuals::dark());
    ctx.all_styles_mut(|s| {
        s.visuals.panel_fill = Color32::TRANSPARENT;
        s.visuals.window_fill = Color32::from_rgb(0x2a, 0x2a, 0x2e);
        s.visuals.override_text_color = Some(TEXT);
        s.visuals.selection.bg_fill = ACCENT;
        s.visuals.hyperlink_color = GOLD;
        s.visuals.extreme_bg_color = Color32::from_white_alpha(14);
        for w in [&mut s.visuals.widgets.inactive, &mut s.visuals.widgets.hovered, &mut s.visuals.widgets.active] {
            w.corner_radius = CornerRadius::same(10);
        }
        s.visuals.widgets.inactive.weak_bg_fill = Color32::from_white_alpha(18);
        s.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
        s.visuals.widgets.hovered.weak_bg_fill = Color32::from_white_alpha(30);
        s.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_white_alpha(40));
        s.spacing.button_padding = Vec2::new(14.0, 6.0);
        s.spacing.item_spacing = Vec2::new(8.0, 8.0);
        // Labels mustn't swallow clicks meant for the clickable tiles.
        s.interaction.selectable_labels = false;
    });
}

fn paint_background(ui: &egui::Ui) {
    let r = ui.max_rect();
    let p = ui.painter();
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(r.left_top(), BG_TOP);
    mesh.colored_vertex(r.right_top(), BG_TOP);
    mesh.colored_vertex(r.left_bottom(), BG_BOTTOM);
    mesh.colored_vertex(r.right_bottom(), BG_BOTTOM);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    p.add(mesh);
    // A soft red glow behind the header, like light through the glass.
    let c = r.left_top() + Vec2::new(90.0, 40.0);
    for i in 0..24 {
        let t = i as f32 / 24.0;
        p.circle_filled(c, 320.0 * (1.0 - t), Color32::from_rgba_unmultiplied(0xff, 0x45, 0x3a, (1.0 + 2.0 * t) as u8));
    }
}

fn accent_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).color(Color32::WHITE).strong())
            .fill(ACCENT)
            .corner_radius(CornerRadius::same(10))
            .min_size(Vec2::new(92.0, 30.0)),
    )
}

fn plain_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(egui::Button::new(RichText::new(text).color(TEXT)).corner_radius(CornerRadius::same(10)).min_size(Vec2::new(80.0, 30.0)))
}

fn glass_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_white_alpha(11))
        .stroke(Stroke::new(1.0, Color32::from_white_alpha(22)))
        .corner_radius(CornerRadius::same(18))
        .inner_margin(Margin::same(16))
}

enum Action {
    Install(Release),
    Open(std::path::PathBuf),
    Uninstall(Release),
    ShowFiles(std::path::PathBuf),
    Dismiss(String),
    Details(String),
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.take_decoded(&ctx);
        paint_background(ui);
        let (manifest, note, loading, state, jobs, queue_busy) = {
            let s = self.shared.lock().unwrap();
            (s.manifest.clone(), s.note.clone(), s.loading, s.state.clone(), s.jobs.clone(), s.queue_busy)
        };
        let mut actions = Vec::new();
        let apps: Vec<Release> = manifest.as_ref().map(|m| m.apps.clone()).unwrap_or_default();
        let updatable: Vec<Release> = apps
            .iter()
            .filter(|a| a.asset().is_some() && state.apps.get(&a.id).is_some_and(|i| i.build < a.build))
            .cloned()
            .collect();

        egui::Frame::new().inner_margin(Margin::symmetric(26, 22)).show(ui, |ui| {
            // Header
            ui.horizontal(|ui| {
                if let Some(t) = self.icon(&ctx, "studio") {
                    ui.add(egui::Image::new(&t).fit_to_exact_size(Vec2::splat(46.0)));
                }
                ui.vertical(|ui| {
                    ui.add_space(2.0);
                    ui.label(RichText::new("RNG Studio").font(FontId::proportional(24.0)).strong());
                    ui.label(RichText::new("RNG Media apps").color(DIM));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add_enabled(!loading, egui::Button::new(RichText::new("Check for updates").color(TEXT)).corner_radius(CornerRadius::same(10)).min_size(Vec2::new(0.0, 30.0))).clicked() {
                        self.shots.retain(|_, s| !matches!(s, Shot::Failed));
                        self.refresh(&ctx);
                    }
                    if !updatable.is_empty() && !queue_busy && accent_button(ui, &format!("Update all ({})", updatable.len())).clicked() {
                        self.run_queue(&ctx, updatable.clone());
                    }
                });
            });
            ui.add_space(14.0);

            // RNG Studio's own update
            if let Some(rel) = manifest.as_ref().and_then(|m| m.launcher.clone()).filter(|l| l.build > store::BUILD && l.asset().is_some()) {
                glass_frame().stroke(Stroke::new(1.0, ACCENT.gamma_multiply(0.6))).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        match jobs.get("studio") {
                            Some(Job::Busy { label, frac }) => {
                                ui.add(egui::ProgressBar::new(frac.unwrap_or(0.0)).text(label.clone()).fill(ACCENT));
                            }
                            other => {
                                if let Some(Job::Failed(e)) = other {
                                    ui.colored_label(ACCENT, e);
                                } else {
                                    ui.label(format!("A new RNG Studio is ready (v{}).", rel.version));
                                }
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if accent_button(ui, "Update and restart").clicked() {
                                        self.self_update(&ctx, rel.clone());
                                    }
                                });
                            }
                        }
                    });
                });
                ui.add_space(12.0);
            }

            if apps.is_empty() {
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    if loading {
                        ui.spinner();
                        ui.label(RichText::new("Fetching the app list…").color(DIM));
                    } else {
                        ui.label(RichText::new(note.clone().unwrap_or_else(|| "No apps listed yet.".into())).color(DIM));
                        if plain_button(ui, "Try again").clicked() {
                            self.refresh(&ctx);
                        }
                    }
                });
                return;
            }

            if let Some(app) = self.detail.as_ref().and_then(|id| apps.iter().find(|a| &a.id == id)).cloned() {
                self.detail_view(ui, &ctx, &app, state.apps.get(&app.id), jobs.get(&app.id), queue_busy, &mut actions);
                return;
            }
            self.detail = None;

            egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
                let cols = ((ui.available_width() + 14.0) / 380.0).floor().clamp(1.0, 3.0) as usize;
                for row in apps.chunks(cols) {
                    ui.columns(cols, |columns| {
                        for (col, app) in columns.iter_mut().zip(row) {
                            self.tile(col, &ctx, app, state.apps.get(&app.id), jobs.get(&app.id), queue_busy, &mut actions);
                        }
                    });
                    ui.add_space(6.0);
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if let Some(n) = &note {
                        ui.label(RichText::new(n).color(DIM).small());
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.hyperlink_to(RichText::new("rng.media").small(), "https://www.rng.media");
                        if ui.link(RichText::new("About & licences").small()).clicked() {
                            self.show_about = true;
                        }
                        ui.label(RichText::new(format!("RNG Studio {} · build {}", store::VERSION, store::BUILD)).color(DIM).small());
                    });
                });
            });
        });

        if self.zoom_first {
            if let Some(u) = self.shots.iter().find(|(_, s)| matches!(s, Shot::Ready(_))).map(|(u, _)| u.clone()) {
                self.zoom = Some(u);
                self.zoom_first = false;
            }
        }
        self.zoom_overlay(&ctx);
        self.test_shot(&ctx);

        if self.show_about {
            let mut open = true;
            egui::Window::new("About RNG Studio")
                .open(&mut open)
                .collapsible(false)
                .default_size([560.0, 440.0])
                .show(&ctx, |ui| {
                    ui.label(RichText::new(format!("RNG Studio {} (build {})", store::VERSION, store::BUILD)).strong());
                    ui.label("© RNG Media S.L. Installs, opens and updates RNG Media's apps. It contacts only www.rng.media and github.com/AndreRNG/rng-studio, to read the app list and download apps.");
                    ui.label(RichText::new("The apps it installs are based on open-source projects (MIT OR Apache-2.0); each one includes its own licence, notices and credits in its About screen and install folder.").color(DIM));
                    ui.add_space(6.0);
                    ui.label(RichText::new("Open-source components in RNG Studio").strong());
                    // ~10k lines: lay out only the rows on screen.
                    static LINES: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
                    let lines = LINES.get_or_init(|| THIRD_PARTY.lines().collect());
                    let font = FontId::monospace(10.5);
                    let row_h = ui.fonts_mut(|f| f.row_height(&font));
                    egui::ScrollArea::vertical().max_height(300.0).auto_shrink([false, true]).show_rows(ui, row_h, lines.len(), |ui, range| {
                        for l in &lines[range] {
                            ui.label(RichText::new(*l).font(font.clone()).color(DIM));
                        }
                    });
                });
            self.show_about = open;
        }

        for a in actions {
            match a {
                Action::Install(r) => self.run_queue(&ctx, vec![r]),
                Action::Open(p) => {
                    if let Err(e) = store::launch(&p) {
                        self.shared.lock().unwrap().note = Some(e);
                    }
                }
                Action::Uninstall(r) => {
                    let mut s = self.shared.lock().unwrap();
                    match store::uninstall(&r) {
                        Ok(()) => {
                            s.state.apps.remove(&r.id);
                        }
                        Err(e) => {
                            s.jobs.insert(r.id.clone(), Job::Failed(e));
                        }
                    }
                }
                Action::ShowFiles(p) => store::open_folder(&p),
                Action::Dismiss(id) => {
                    self.shared.lock().unwrap().jobs.remove(&id);
                }
                Action::Details(id) => self.detail = Some(id),
            }
        }
    }
}

impl App {
    #[allow(clippy::too_many_arguments)]
    fn tile(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, app: &Release, inst: Option<&Installed>, job: Option<&Job>, queue_busy: bool, actions: &mut Vec<Action>) {
        // The whole tile opens the detail view; its buttons sit on top and keep their own clicks.
        let resp = ui
            .scope_builder(egui::UiBuilder::new().sense(egui::Sense::click()), |ui| {
                let lit = ui.response().contains_pointer();
                let frame = if lit { glass_frame().fill(Color32::from_white_alpha(16)).stroke(Stroke::new(1.0, Color32::from_white_alpha(38))) } else { glass_frame() };
                frame.show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_min_height(112.0);
                    ui.horizontal(|ui| {
                        self.app_icon(ui, ctx, app, 80.0);
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&app.name).font(FontId::proportional(19.0)).strong());
                            ui.label(RichText::new(&app.tagline).color(DIM));
                            version_line(ui, app, inst);
                            ui.add_space(4.0);
                            app_buttons(ui, app, inst, job, queue_busy, actions);
                        });
                    });
                });
            })
            .response;
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            actions.push(Action::Details(app.id.clone()));
        }
    }

    fn app_icon(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, app: &Release, size: f32) {
        match self.icon(ctx, &app.id) {
            Some(t) => {
                ui.add(egui::Image::new(&t).fit_to_exact_size(Vec2::splat(size)));
            }
            None => {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::same((size / 4.0) as u8), Color32::from_white_alpha(20));
                ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, app.name.chars().next().unwrap_or('?'), FontId::proportional(size * 0.45), TEXT);
            }
        }
    }

    /// One app, the way its page on rng.media shows it: pitch, buttons, what it does, features,
    /// screenshots and the open-source credit. The copy comes from the manifest's `about` block.
    #[allow(clippy::too_many_arguments)]
    fn detail_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, app: &Release, inst: Option<&Installed>, job: Option<&Job>, queue_busy: bool, actions: &mut Vec<Action>) {
        let about = app.about.clone().unwrap_or_default();
        if ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.pointer.button_pressed(egui::PointerButton::Extra1)) && self.zoom.is_none() {
            self.detail = None;
        }
        if plain_button(ui, "‹  All apps").clicked() {
            self.detail = None;
        }
        ui.add_space(6.0);
        let mut scroll = egui::ScrollArea::vertical().id_salt(("detail", &app.id)).auto_shrink([false, false]);
        if let Some(y) = self.scroll_to.take() {
            scroll = scroll.vertical_scroll_offset(y);
        }
        scroll.show(ui, |ui| {
            ui.set_width(ui.available_width());
            // Hero
            glass_frame().inner_margin(Margin::same(22)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    self.app_icon(ui, ctx, app, 128.0);
                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(app.tagline.to_uppercase()).color(ACCENT).small().strong());
                        ui.label(RichText::new(&app.name).font(FontId::proportional(32.0)).strong());
                        if !about.pitch.is_empty() {
                            ui.label(RichText::new(&about.pitch).font(FontId::proportional(17.0)));
                        }
                        version_line(ui, app, inst);
                        ui.add_space(4.0);
                        app_buttons(ui, app, inst, job, queue_busy, actions);
                    });
                });
                if !about.intro.is_empty() {
                    ui.add_space(10.0);
                    ui.label(RichText::new(&about.intro).color(DIM));
                }
            });

            if !about.description.is_empty() {
                section(ui, "What it does");
                ui.label(&about.description);
            }

            if !about.features.is_empty() {
                section(ui, "Standout features");
                let cols = if ui.available_width() > 640.0 { 2 } else { 1 };
                for row in about.features.chunks(cols) {
                    ui.columns(cols, |columns| {
                        for (col, f) in columns.iter_mut().zip(row) {
                            // `columns` lays out justified, which stretches wrapped text; left-align it.
                            col.with_layout(egui::Layout::top_down(egui::Align::LEFT), |col| glass_frame().inner_margin(Margin::same(14)).show(col, |ui| {
                                ui.set_width(ui.available_width());
                                ui.set_min_height(58.0);
                                ui.horizontal(|ui| {
                                    let (r, _) = ui.allocate_exact_size(Vec2::new(3.0, 18.0), egui::Sense::hover());
                                    ui.painter().rect_filled(r, CornerRadius::same(2), ACCENT);
                                    ui.label(RichText::new(&f.title).strong());
                                });
                                ui.label(RichText::new(&f.body).color(DIM));
                            }));
                        }
                    });
                    ui.add_space(2.0);
                }
            }

            if !about.screenshots.is_empty() {
                section(ui, "Screenshots");
                self.gallery(ui, ctx, &about);
            }

            ui.add_space(18.0);
            ui.separator();
            ui.add_space(6.0);
            let credit = if about.credit.is_empty() { format!("{} is based on an open-source project by the ArtCraft team, licensed MIT OR Apache-2.0, and modified by RNG Media S.L.", app.name) } else { about.credit.clone() };
            ui.label(RichText::new(credit).color(DIM).size(12.5));
            ui.horizontal(|ui| {
                let page = if about.page.is_empty() { "https://www.rng.media/studio".to_string() } else { about.page.clone() };
                let licences = if about.licences.is_empty() { "https://github.com/AndreRNG/rng-studio/tree/main/licences".to_string() } else { about.licences.clone() };
                ui.hyperlink_to(RichText::new("Licences and notices").small(), licences);
                ui.label(RichText::new("·").color(DIM).small());
                ui.hyperlink_to(RichText::new(format!("{} on rng.media", app.name)).small(), page);
            });
            ui.add_space(8.0);
        });
    }

    fn gallery(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, about: &About) {
        let cols = if ui.available_width() > 640.0 { 2 } else { 1 };
        for s in &about.screenshots {
            self.request_shot(ctx, &s.url);
        }
        for row in about.screenshots.chunks(cols) {
            ui.columns(cols, |columns| {
                for (col, s) in columns.iter_mut().zip(row) {
                    col.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        let w = ui.available_width();
                        let aspect = if s.width > 0 && s.height > 0 { s.height as f32 / s.width as f32 } else { 0.625 };
                        // Wide strips (the CutCat timeline) keep the row's 16:10 box and sit centred in it.
                        let box_h = w * 0.625;
                        let (r, resp) = ui.allocate_exact_size(Vec2::new(w, box_h), egui::Sense::click());
                        let p = ui.painter();
                        p.rect_filled(r, CornerRadius::same(12), Color32::from_white_alpha(10));
                        match self.shots.get(&s.url) {
                            Some(Shot::Ready(t)) => {
                                let tex = t.size_vec2();
                                let a = if tex.x > 0.0 { tex.y / tex.x } else { aspect };
                                let size = if a < 0.625 { Vec2::new(w, w * a) } else { Vec2::new(box_h / a, box_h) };
                                let ir = egui::Rect::from_center_size(r.center(), size);
                                egui::Image::new(t).corner_radius(CornerRadius::same(12)).paint_at(ui, ir);
                                if resp.hovered() {
                                    ui.painter().rect_stroke(r, CornerRadius::same(12), Stroke::new(1.5, ACCENT.gamma_multiply(0.8)), egui::StrokeKind::Inside);
                                }
                                if resp.on_hover_cursor(egui::CursorIcon::ZoomIn).clicked() {
                                    self.zoom = Some(s.url.clone());
                                }
                            }
                            Some(Shot::Failed) => {
                                p.text(r.center(), egui::Align2::CENTER_CENTER, "Screenshot not available offline", FontId::proportional(13.0), DIM);
                            }
                            _ => {
                                let sp = 22.0;
                                egui::Spinner::new().size(sp).color(DIM).paint_at(ui, egui::Rect::from_center_size(r.center(), Vec2::splat(sp)));
                            }
                        }
                        if !s.title.is_empty() {
                            ui.label(RichText::new(&s.title).strong());
                        }
                        if !s.caption.is_empty() {
                            ui.label(RichText::new(&s.caption).color(DIM));
                        }
                    });
                }
            });
            ui.add_space(10.0);
        }
    }

    fn test_shot(&mut self, ctx: &egui::Context) {
        let Some((path, frames)) = self.shot_to.as_mut() else { return };
        *frames += 1;
        let settled = !self.shots.values().any(|s| matches!(s, Shot::Loading)) && !self.zoom_first;
        if *frames == 400 || (*frames > 30 && settled && *frames < 400) {
            *frames = 400;
            let (path, c) = (path.clone(), ctx.clone());
            ctx.request_screenshot(move |img| {
                let _ = image::save_buffer(&path, img.as_raw(), img.width() as u32, img.height() as u32, image::ColorType::Rgba8);
                c.send_viewport_cmd(egui::ViewportCommand::Close);
                c.request_repaint();
            });
        }
        ctx.request_repaint();
    }

    fn zoom_overlay(&mut self, ctx: &egui::Context) {
        let Some(url) = self.zoom.clone() else { return };
        let Some(Shot::Ready(t)) = self.shots.get(&url) else {
            self.zoom = None;
            return;
        };
        let t = t.clone();
        let screen = ctx.content_rect();
        let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        egui::Area::new(egui::Id::unique("rng-studio-zoom")).order(egui::Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
            let (r, resp) = ui.allocate_exact_size(screen.size(), egui::Sense::click());
            ui.painter().rect_filled(r, CornerRadius::ZERO, Color32::from_black_alpha(225));
            let room = r.shrink(28.0);
            let tex = t.size_vec2();
            let scale = (room.width() / tex.x).min(room.height() / tex.y).min(1.5);
            egui::Image::new(&t).corner_radius(CornerRadius::same(10)).paint_at(ui, egui::Rect::from_center_size(r.center(), tex * scale));
            let x = egui::Rect::from_center_size(r.right_top() + Vec2::new(-26.0, 26.0), Vec2::splat(12.0));
            let st = Stroke::new(2.0, TEXT);
            ui.painter().line_segment([x.left_top(), x.right_bottom()], st);
            ui.painter().line_segment([x.right_top(), x.left_bottom()], st);
            close |= resp.clicked();
        });
        if close {
            self.zoom = None;
        }
    }
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(18.0);
    ui.label(RichText::new(title).font(FontId::proportional(19.0)).strong());
    ui.add_space(2.0);
}

fn version_line(ui: &mut egui::Ui, app: &Release, inst: Option<&Installed>) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        match inst {
            Some(i) if i.build < app.build => {
                ui.label(RichText::new(format!("v{}", i.version)).color(GOLD).monospace());
                ui.label(RichText::new(format!("→ v{} available", app.version)).color(DIM).small());
            }
            Some(i) => {
                ui.label(RichText::new(format!("v{}", i.version)).color(GOLD).monospace());
                ui.label(RichText::new("installed").color(DIM).small());
            }
            None => {
                ui.label(RichText::new(format!("v{}", app.version)).color(DIM).monospace());
            }
        }
    });
}

/// Install / Update / Open (and the ⋯ menu), or the progress bar while a job runs.
fn app_buttons(ui: &mut egui::Ui, app: &Release, inst: Option<&Installed>, job: Option<&Job>, queue_busy: bool, actions: &mut Vec<Action>) {
    match job {
        Some(Job::Busy { label, frac }) => {
            let bar = egui::ProgressBar::new(frac.unwrap_or(0.0)).text(RichText::new(label).small()).fill(ACCENT).desired_height(18.0);
            ui.add(if frac.is_none() { bar.animate(true) } else { bar });
        }
        _ => {
            ui.horizontal(|ui| {
                let can = app.asset().is_some();
                match inst {
                    _ if !can && inst.is_none() => {
                        ui.add_enabled(false, egui::Button::new("Coming soon").min_size(Vec2::new(92.0, 30.0)));
                    }
                    None => {
                        if accent_button(ui, "Install").clicked() && !queue_busy {
                            actions.push(Action::Install(app.clone()));
                        }
                    }
                    Some(i) => {
                        if can && i.build < app.build {
                            if accent_button(ui, "Update").clicked() && !queue_busy {
                                actions.push(Action::Install(app.clone()));
                            }
                            if plain_button(ui, "Open").clicked() {
                                actions.push(Action::Open(i.exe.clone()));
                            }
                        } else if accent_button(ui, "Open").clicked() {
                            actions.push(Action::Open(i.exe.clone()));
                        }
                        ui.menu_button(RichText::new("⋯").strong(), |ui| {
                            if ui.button("Show files").clicked() {
                                actions.push(Action::ShowFiles(i.dir.clone()));
                                ui.close();
                            }
                            if ui.button("Reinstall").clicked() && !queue_busy {
                                actions.push(Action::Install(app.clone()));
                                ui.close();
                            }
                            if ui.button(RichText::new("Uninstall").color(ACCENT)).clicked() {
                                actions.push(Action::Uninstall(app.clone()));
                                ui.close();
                            }
                        });
                    }
                }
            });
            if let Some(Job::Failed(e)) = job {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(e).color(ACCENT).small());
                    if ui.small_button("OK").clicked() {
                        actions.push(Action::Dismiss(app.id.clone()));
                    }
                });
            }
        }
    }
}

/// `rng-studio --install <id>` / `--list`: the same work without a window (scripts, tests).
fn cli(args: &[String]) -> Option<i32> {
    let (m, note) = match store::fetch_manifest() {
        Ok(x) => x,
        Err(e) => {
            eprintln!("{e}");
            return Some(1);
        }
    };
    if let Some(n) = note {
        eprintln!("{n}");
    }
    let state = store::load_state();
    match args.first().map(String::as_str) {
        Some("--list") => {
            for a in &m.apps {
                let inst = state.apps.get(&a.id).map(|i| format!("installed v{} b{}", i.version, i.build)).unwrap_or_default();
                println!("{:10} v{} b{} {} {}", a.id, a.version, a.build, if a.asset().is_some() { "available" } else { "-" }, inst);
            }
            Some(0)
        }
        Some("--install") => {
            let ids: Vec<&String> = args[1..].iter().collect();
            let mut code = 0;
            for a in m.apps.iter().filter(|a| ids.iter().any(|i| *i == &a.id || i.as_str() == "all")) {
                if ids.iter().any(|i| i.as_str() == "all") && state.apps.get(&a.id).is_some_and(|i| i.build >= a.build) {
                    continue;
                }
                match store::install(a, &|_, _| {}) {
                    Ok(i) => println!("{} v{} -> {}", a.name, a.version, i.exe.display()),
                    Err(e) => {
                        eprintln!("{}: {e}", a.name);
                        code = 1;
                    }
                }
            }
            Some(code)
        }
        _ => None,
    }
}

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a.starts_with("--")) {
        if let Some(code) = cli(&args) {
            std::process::exit(code);
        }
    }
    if store::self_install() {
        return Ok(());
    }
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("RNG Studio")
        .with_inner_size([840.0, 560.0])
        .with_min_inner_size([440.0, 360.0])
        .with_app_id("media.rng.studio");
    if let Ok(icon) = eframe::icon_data::from_png_bytes(ICON_BIG) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions { viewport, renderer: eframe::Renderer::Glow, ..Default::default() };
    eframe::run_native("RNG Studio", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
