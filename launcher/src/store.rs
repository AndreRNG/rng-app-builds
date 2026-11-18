//! Manifest, local state, downloads and installs. No UI in here.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const MANIFEST_URL: &str = "https://www.rng.media/apps/manifest.json";
pub const BUILD: u64 = match option_env!("RNG_STUDIO_BUILD") {
    Some(s) => parse_u64(s),
    None => 0,
};
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

const fn parse_u64(s: &str) -> u64 {
    let b = s.as_bytes();
    let mut i = 0;
    let mut n = 0u64;
    while i < b.len() {
        n = n * 10 + (b[i] - b'0') as u64;
        i += 1;
    }
    n
}

pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Asset {
    pub url: String,
    pub sha256: String,
    #[serde(default)]
    pub size: u64,
    /// Path of the program inside the archive (".app" bundle on macOS).
    #[serde(default)]
    pub exe: String,
    /// Installer kind (apps that install and update themselves): the script in the archive to run.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub installer: String,
    /// Run `installer` in a terminal window (Linux).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub terminal: bool,
    /// A file that exists once the app is installed (~, %APPDATA% and %LOCALAPPDATA% are expanded).
    /// Apps with one are never tracked or updated by RNG Studio: they update themselves.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub detect: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Release {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub tagline: String,
    /// Window class / Wayland app id, so the dock groups the window with its menu entry.
    #[serde(default)]
    pub wm_class: String,
    pub version: String,
    pub build: u64,
    #[serde(default)]
    pub platforms: BTreeMap<String, Asset>,
    /// Like `platforms`, for apps that need this launcher's installer support. Launchers older than
    /// that don't read it, so they list the app as "Coming soon" instead of installing it wrongly.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub installers: BTreeMap<String, Asset>,
    /// Icon URL for apps without one built into the launcher.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon: String,
    /// Copy for the app's detail view (newer manifests only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<About>,
    /// Earlier ids and names of this app (it was renamed): installs recorded under them carry over.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replaces: Vec<Former>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Former {
    pub id: String,
    #[serde(default)]
    pub name: String,
}

/// Renamed apps: installs recorded under a former id move to the app's current id, and their
/// menu / Start menu entry is rewritten under the new name. The files stay where they are until the
/// next update. Returns true when the state changed (it is saved here).
pub fn carry_over_renames(state: &mut State, manifest: &Manifest) -> bool {
    let mut changed = false;
    for app in &manifest.apps {
        for old in &app.replaces {
            if old.id == app.id || state.apps.contains_key(&app.id) {
                continue;
            }
            let Some(i) = state.apps.remove(&old.id) else { continue };
            shortcuts::remove_former(old);
            shortcuts::create(app, &i);
            state.apps.insert(app.id.clone(), i);
            changed = true;
        }
    }
    if changed {
        let _ = save_state(state);
    }
    changed
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct About {
    #[serde(default)]
    pub pitch: String,
    #[serde(default)]
    pub intro: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub features: Vec<Feature>,
    #[serde(default)]
    pub screenshots: Vec<Screenshot>,
    /// The app's page on rng.media.
    #[serde(default)]
    pub page: String,
    /// Plain-text open-source credit.
    #[serde(default)]
    pub credit: String,
    #[serde(default)]
    pub licences: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Feature {
    pub title: String,
    #[serde(default)]
    pub body: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Screenshot {
    pub url: String,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub caption: String,
}

impl Release {
    pub fn asset(&self) -> Option<&Asset> {
        platform_keys().iter().find_map(|k| self.platforms.get(*k).or_else(|| self.installers.get(*k)))
    }

    /// For apps that update themselves: whether they're installed on this computer, found by their
    /// `detect` file rather than RNG Studio's own records.
    pub fn detected(&self) -> Option<Installed> {
        let a = self.asset().filter(|a| !a.detect.is_empty())?;
        let p = expand(&a.detect);
        p.exists().then(|| Installed { version: String::new(), build: self.build, exe: p, dir: data_dir().join("apps").join(&self.id) })
    }

    pub fn self_updating(&self) -> bool {
        self.asset().is_some_and(|a| !a.detect.is_empty())
    }
}

/// `~/x`, `%APPDATA%\x`, `%LOCALAPPDATA%\x` → full paths.
pub fn expand(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        return dirs::home_dir().unwrap_or_default().join(rest);
    }
    for (var, dir) in [("%APPDATA%", dirs::data_dir()), ("%LOCALAPPDATA%", dirs::data_local_dir())] {
        if let Some(rest) = p.strip_prefix(var) {
            return dir.unwrap_or_default().join(rest.trim_start_matches(['\\', '/']));
        }
    }
    PathBuf::from(p)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Manifest {
    pub launcher: Option<Release>,
    #[serde(default)]
    pub apps: Vec<Release>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Installed {
    pub version: String,
    pub build: u64,
    pub exe: PathBuf,
    #[serde(default)]
    pub dir: PathBuf,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    pub apps: BTreeMap<String, Installed>,
}

/// Platform keys this machine can run, most specific first.
pub fn platform_keys() -> &'static [&'static str] {
    if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        &["linux-x86_64"]
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        &["linux-aarch64"]
    } else if cfg!(target_os = "windows") && cfg!(target_arch = "aarch64") {
        &["windows-aarch64", "windows-x86_64"]
    } else if cfg!(target_os = "windows") {
        &["windows-x86_64"]
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        &["macos-aarch64", "macos-universal"]
    } else if cfg!(target_os = "macos") {
        &["macos-x86_64", "macos-universal"]
    } else {
        &[]
    }
}

/// `<local data>/RNG Studio`. Installs made while the app was called LaunchLlama keep their
/// `LaunchLlama` folder (it may hold the running program and open apps, so it is never moved).
pub fn data_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(std::env::temp_dir);
    let legacy = base.join("LaunchLlama");
    if legacy.exists() {
        legacy
    } else {
        base.join("RNG Studio")
    }
}

fn state_path() -> PathBuf {
    data_dir().join("state.json")
}

pub fn load_state() -> State {
    fs::read(state_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

pub fn save_state(s: &State) -> Result<()> {
    write_atomic(&state_path(), &serde_json::to_vec_pretty(s).unwrap())
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes).map_err(|e| format!("write {}: {e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(15)))
        .user_agent(format!("RNGStudio/{VERSION}"))
        .build()
        .into()
}

/// Reads the manifest from `RNG_STUDIO_MANIFEST` (URL or file path) or rng.media.
/// A good copy is cached so the window still lists the apps offline.
pub fn fetch_manifest() -> Result<(Manifest, Option<String>)> {
    let src = std::env::var("RNG_STUDIO_MANIFEST").unwrap_or_else(|_| MANIFEST_URL.into());
    let cache = data_dir().join("manifest.json");
    let fetched: Result<String> = if src.starts_with("http") {
        agent()
            .get(&src)
            .call()
            .map_err(|e| format!("can't reach {src}: {e}"))
            .and_then(|mut r| r.body_mut().read_to_string().map_err(|e| e.to_string()))
    } else {
        fs::read_to_string(&src).map_err(|e| format!("{src}: {e}"))
    };
    let local = !src.starts_with("http");
    match fetched.and_then(|t| parse_manifest(&t, local).map(|m| (t, m))) {
        Ok((text, m)) => {
            let _ = write_atomic(&cache, text.as_bytes());
            Ok((m, None))
        }
        Err(e) => match fs::read_to_string(&cache).ok().and_then(|t| parse_manifest(&t, false).ok()) {
            Some(m) => Ok((m, Some(format!("Showing the last good app list ({e})")))),
            None => Err(e),
        },
    }
}

/// Ed25519 key that signs manifest.json (private half: ~/.config/rng-studio-release on Andre's PC).
const MANIFEST_KEY: [u8; 32] = hex_32("30cbdc1ad7e4630a7c4b94c4e0ca2b2e8295bf9e0c9f85a990af54c6d7eeb119");
/// While false, unsigned manifests are still accepted (the grace period); a bad signature never is.
const REQUIRE_SIGNED: bool = false;

const fn hex_32(s: &str) -> [u8; 32] {
    let b = s.as_bytes();
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        out[i] = hex_digit(b[2 * i]) << 4 | hex_digit(b[2 * i + 1]);
        i += 1;
    }
    out
}

const fn hex_digit(c: u8) -> u8 {
    if c <= b'9' {
        c - b'0'
    } else {
        c - b'a' + 10
    }
}

#[derive(Deserialize)]
struct Signed {
    body: String,
    sig: String,
}

/// Reads a manifest, using its `signed` copy when it has one: that copy must carry a valid
/// signature from MANIFEST_KEY, so a tampered download is refused. `local` = a test file.
fn parse_manifest(text: &str, local: bool) -> Result<Manifest> {
    #[derive(Deserialize)]
    struct Outer {
        signed: Option<Signed>,
    }
    let outer: Outer = serde_json::from_str(text).map_err(|e| format!("bad manifest: {e}"))?;
    match outer.signed {
        Some(s) => {
            use ed25519_dalek::{Signature, Verifier, VerifyingKey};
            let key = VerifyingKey::from_bytes(&MANIFEST_KEY).map_err(|e| e.to_string())?;
            let sig: [u8; 64] = hex::decode(s.sig.trim()).ok().and_then(|v| v.try_into().ok()).ok_or("the app list's signature is unreadable")?;
            key.verify(s.body.as_bytes(), &Signature::from_bytes(&sig)).map_err(|_| "the app list failed its signature check, so it wasn't used".to_string())?;
            serde_json::from_str(&s.body).map_err(|e| format!("bad manifest: {e}"))
        }
        None if REQUIRE_SIGNED && !local => Err("the app list isn't signed, so it wasn't used".into()),
        None => serde_json::from_str(text).map_err(|e| format!("bad manifest: {e}")),
    }
}

/// A screenshot's bytes, from `<data>/screens` or downloaded (http) / read (path) and kept there,
/// so the detail view still has its pictures offline.
pub fn screenshot(url: &str) -> Result<Vec<u8>> {
    let key = hex::encode(&Sha256::digest(url.as_bytes())[..12]);
    let ext = url.rsplit('.').next().filter(|e| e.len() <= 4 && !e.contains('/')).unwrap_or("img");
    let path = data_dir().join("screens").join(format!("{key}.{ext}"));
    if let Ok(b) = fs::read(&path) {
        return Ok(b);
    }
    let bytes = if url.starts_with("http") {
        let mut r = agent().get(url).call().map_err(|e| e.to_string())?;
        r.body_mut().with_config().limit(16 * 1024 * 1024).read_to_vec().map_err(|e| e.to_string())?
    } else {
        fs::read(url).map_err(|e| format!("{url}: {e}"))?
    };
    // Best effort: a failed write only means it's fetched again next time.
    let tmp = path.with_extension(format!("{:?}.part", std::thread::current().id()).replace(['(', ')'], ""));
    let _ = fs::create_dir_all(path.parent().unwrap());
    let _ = fs::write(&tmp, &bytes).and_then(|_| fs::rename(&tmp, &path));
    let _ = fs::remove_file(&tmp);
    Ok(bytes)
}

/// Downloads `asset` into `dest_dir`, checking size and SHA-256. `progress(done, total)`.
pub fn download(asset: &Asset, dest_dir: &Path, progress: &dyn Fn(u64, u64)) -> Result<PathBuf> {
    fs::create_dir_all(dest_dir).map_err(|e| e.to_string())?;
    let name = asset.url.rsplit('/').next().unwrap_or("download").split('?').next().unwrap().to_string();
    let path = dest_dir.join(&name);
    let part = dest_dir.join(format!("{name}.part"));
    let mut reader: Box<dyn Read> = if asset.url.starts_with("http") {
        let resp = agent().get(&asset.url).call().map_err(|e| format!("download failed: {e}"))?;
        Box::new(resp.into_body().into_reader())
    } else {
        Box::new(fs::File::open(&asset.url).map_err(|e| format!("{}: {e}", asset.url))?)
    };
    let mut out = fs::File::create(&part).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    let mut done = 0u64;
    loop {
        let n = reader.read(&mut buf).map_err(|e| format!("download interrupted: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        progress(done, asset.size.max(done));
    }
    out.sync_all().ok();
    drop(out);
    let got = hex::encode(hasher.finalize());
    if !got.eq_ignore_ascii_case(asset.sha256.trim()) {
        let _ = fs::remove_file(&part);
        return Err("download didn't match its checksum; nothing was installed".into());
    }
    fs::rename(&part, &path).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Unpacks a .tar.gz or .zip into `dest` (which must not exist yet).
pub fn extract(archive: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let f = fs::File::open(archive).map_err(|e| e.to_string())?;
    let name = archive.to_string_lossy();
    if !(name.ends_with(".tar.gz") || name.ends_with(".tgz") || name.ends_with(".zip")) {
        // A bare program (RNG Studio.exe): nothing to unpack.
        drop(f);
        let out = dest.join(archive.file_name().unwrap());
        fs::copy(archive, &out).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&out, fs::Permissions::from_mode(0o755));
        }
        return Ok(());
    }
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        let mut t = tar::Archive::new(flate2::read::GzDecoder::new(f));
        t.set_preserve_permissions(true);
        t.unpack(dest).map_err(|e| format!("unpack: {e}"))
    } else {
        let mut z = zip::ZipArchive::new(f).map_err(|e| format!("unpack: {e}"))?;
        for i in 0..z.len() {
            let mut entry = z.by_index(i).map_err(|e| e.to_string())?;
            let Some(rel) = entry.enclosed_name() else { continue };
            let out = dest.join(rel);
            if entry.is_dir() {
                fs::create_dir_all(&out).map_err(|e| e.to_string())?;
                continue;
            }
            if let Some(p) = out.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            #[cfg(unix)]
            if entry.is_symlink() {
                let mut target = String::new();
                entry.read_to_string(&mut target).map_err(|e| e.to_string())?;
                let _ = fs::remove_file(&out);
                std::os::unix::fs::symlink(target, &out).map_err(|e| e.to_string())?;
                continue;
            }
            let mut w = fs::File::create(&out).map_err(|e| format!("{}: {e}", out.display()))?;
            io::copy(&mut entry, &mut w).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            if let Some(mode) = entry.unix_mode() {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&out, fs::Permissions::from_mode(mode & 0o777));
            }
        }
        Ok(())
    }
}

/// Installs or updates one app. Returns where it now lives.
pub fn install(app: &Release, progress: &dyn Fn(u64, u64)) -> Result<Installed> {
    let asset = app.asset().ok_or("not available for this computer yet")?;
    let root = data_dir();
    let archive = download(asset, &root.join("downloads"), progress)?;
    let dir = root.join("apps").join(&app.id).join(format!("b{}", app.build));
    let staging = dir.with_extension("new");
    let _ = fs::remove_dir_all(&staging);
    extract(&archive, &staging)?;
    let _ = fs::remove_file(&archive);
    let _ = fs::remove_dir_all(&dir);
    fs::rename(&staging, &dir).map_err(|e| e.to_string())?;
    #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
    let mut exe = dir.join(if asset.installer.is_empty() { &asset.exe } else { &asset.installer });
    if !exe.exists() {
        return Err(format!("{} is missing from the download", asset.exe));
    }

    // macOS: the .app goes to ~/Applications so Spotlight and Launchpad find it.
    #[cfg(target_os = "macos")]
    {
        let apps = dirs::home_dir().unwrap().join("Applications");
        fs::create_dir_all(&apps).map_err(|e| e.to_string())?;
        let target = apps.join(exe.file_name().unwrap());
        let old = target.with_extension("app.old");
        let _ = fs::remove_dir_all(&old);
        if target.exists() {
            fs::rename(&target, &old).map_err(|e| e.to_string())?;
        }
        fs::rename(&exe, &target).map_err(|e| e.to_string())?;
        let _ = fs::remove_dir_all(&old);
        exe = target;
    }

    if !asset.installer.is_empty() {
        run_installer(&dir, asset)?;
        return Ok(Installed { version: app.version.clone(), build: app.build, exe, dir });
    }
    if !asset.detect.is_empty() {
        // Installed by moving the .app (macOS); it updates itself from here on.
        return Ok(Installed { version: app.version.clone(), build: app.build, exe, dir });
    }

    let installed = Installed { version: app.version.clone(), build: app.build, exe, dir: dir.clone() };
    shortcuts::create(app, &installed);
    let mut state = load_state();
    let previous = state.apps.insert(app.id.clone(), installed.clone());
    save_state(&state)?;
    // The old copy may still be open (Windows locks it); it is removed on a later start then.
    if let Some(p) = previous {
        if p.dir != dir {
            let _ = fs::remove_dir_all(&p.dir);
            // A renamed app's folder under its former id is empty now.
            if let Some(parent) = p.dir.parent().filter(|d| *d != dir.parent().unwrap()) {
                let _ = fs::remove_dir(parent);
            }
        }
        // macOS, renamed app: the bundle under its former name ("DocDog.app") goes.
        if cfg!(target_os = "macos") && p.exe != installed.exe && p.exe.extension().is_some_and(|e| e == "app") {
            let _ = fs::remove_dir_all(&p.exe);
        }
    }
    clean_stale(&app.id, &dir);
    Ok(installed)
}

/// Removes leftover version folders of an app that are no longer the installed one.
pub fn clean_stale(id: &str, keep: &Path) {
    if let Ok(rd) = fs::read_dir(data_dir().join("apps").join(id)) {
        for e in rd.flatten() {
            if e.path() != keep {
                let _ = fs::remove_dir_all(e.path());
            }
        }
    }
}

pub fn uninstall(app: &Release) -> Result<()> {
    let mut state = load_state();
    if let Some(i) = state.apps.remove(&app.id) {
        if cfg!(target_os = "macos") && i.exe.extension().is_some_and(|e| e == "app") {
            let _ = fs::remove_dir_all(&i.exe);
        }
        fs::remove_dir_all(data_dir().join("apps").join(&app.id)).or_else(|e| if e.kind() == io::ErrorKind::NotFound { Ok(()) } else { Err(e) }).map_err(|e| format!("couldn't remove it (is it still open?): {e}"))?;
        shortcuts::remove(app);
    }
    save_state(&state)
}

/// Runs an app's own installer from its unpacked download and waits until its window closes.
fn run_installer(dir: &Path, asset: &Asset) -> Result<()> {
    let script = dir.join(&asset.installer);
    let cwd = script.parent().unwrap_or(dir).to_path_buf();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&script, fs::Permissions::from_mode(0o755));
    }
    #[cfg(windows)]
    let mut cmd = {
        use std::os::windows::process::CommandExt;
        let mut c = std::process::Command::new("cmd");
        c.arg("/c").arg(&script).creation_flags(0x0000_0010); // CREATE_NEW_CONSOLE
        c
    };
    #[cfg(not(windows))]
    let mut cmd = if asset.terminal && cfg!(target_os = "linux") {
        let sh = ["bash", "-c", "\"$0\"; echo; read -rp 'Press Enter to close this window. ' _"];
        let found = [("konsole", &["--nofork", "-e"][..]), ("kitty", &[][..]), ("gnome-terminal", &["--wait", "--"][..]), ("xfce4-terminal", &["-x"][..]), ("alacritty", &["-e"][..]), ("foot", &[][..]), ("xterm", &["-e"][..])]
            .into_iter()
            .find(|(t, _)| std::process::Command::new("sh").arg("-c").arg(format!("command -v {t}")).stdout(std::process::Stdio::null()).status().is_ok_and(|s| s.success()));
        let Some((term, args)) = found else {
            open_folder(&cwd);
            return Err(format!("no terminal found: run {} from the folder that just opened", asset.installer.rsplit('/').next().unwrap_or("the installer")));
        };
        let mut c = std::process::Command::new(term);
        c.args(args).args(sh).arg(&script);
        c
    } else {
        std::process::Command::new(&script)
    };
    cmd.current_dir(&cwd);
    let status = cmd.status().map_err(|e| format!("couldn't start the installer: {e}"))?;
    if !status.success() {
        return Err("the installer stopped before finishing".into());
    }
    Ok(())
}

/// Opens an app that installed itself, through what its `detect` file points at.
pub fn launch_detected(p: &Path) -> Result<()> {
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext == "desktop" {
        // Run the menu entry's Exec line (field codes like %U dropped).
        let text = fs::read_to_string(p).map_err(|e| e.to_string())?;
        let exec = text.lines().find_map(|l| l.strip_prefix("Exec=")).ok_or("its menu entry has no Exec line")?;
        let exec: String = exec.split_whitespace().filter(|w| !(w.len() == 2 && w.starts_with('%'))).collect::<Vec<_>>().join(" ");
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg(&exec);
        if let Some(home) = dirs::home_dir() {
            cmd.current_dir(home);
        }
        cmd.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        return cmd.spawn().map(|_| ()).map_err(|e| format!("couldn't open it: {e}"));
    }
    if cfg!(windows) {
        // A Start menu shortcut.
        return std::process::Command::new("explorer").arg(p).spawn().map(|_| ()).map_err(|e| format!("couldn't open it: {e}"));
    }
    launch(p)
}

pub fn launch(exe: &Path) -> Result<()> {
    let mut cmd;
    #[cfg(target_os = "macos")]
    {
        cmd = std::process::Command::new("open");
        cmd.arg("-a").arg(exe);
    }
    #[cfg(not(target_os = "macos"))]
    {
        cmd = std::process::Command::new(exe);
        if let Some(home) = dirs::home_dir() {
            cmd.current_dir(home);
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0000_0008 | 0x0000_0200); // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP
    }
    cmd.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    cmd.spawn().map(|_| ()).map_err(|e| format!("couldn't open it: {e}"))
}

pub fn open_folder(p: &Path) {
    let opener = if cfg!(windows) { "explorer" } else if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    let _ = std::process::Command::new(opener).arg(p).spawn();
}

/// Start menu / app menu entries for installed apps.
pub mod shortcuts {
    use super::*;

    #[cfg(target_os = "linux")]
    fn desktop_file(id: &str) -> PathBuf {
        dirs::data_dir().unwrap().join("applications").join(format!("media.rng.studio.{id}.desktop"))
    }

    /// Menu entries written before the rename (media.rng.launchllama.*): rewritten under the new name.
    #[cfg(target_os = "linux")]
    pub fn migrate(apps: &std::collections::BTreeMap<String, Installed>, manifest: &Manifest) {
        let dir = dirs::data_dir().unwrap();
        for (id, i) in apps {
            let old = dir.join("applications").join(format!("media.rng.launchllama.{id}.desktop"));
            if old.exists() {
                if let Some(app) = manifest.apps.iter().find(|a| &a.id == id) {
                    create(app, i);
                    let _ = fs::remove_file(old);
                    let _ = fs::remove_file(dir.join("icons/hicolor/512x512/apps").join(format!("media.rng.launchllama.{id}.png")));
                }
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    pub fn migrate(_: &std::collections::BTreeMap<String, Installed>, _: &Manifest) {}

    #[cfg(target_os = "linux")]
    pub fn create(app: &Release, i: &Installed) {
        let icon_src = i.exe.parent().map(|p| p.join("icon.png"));
        let icon = dirs::data_dir().unwrap().join("icons/hicolor/512x512/apps").join(format!("media.rng.studio.{}.png", app.id));
        if let Some(src) = icon_src.filter(|p| p.exists()) {
            let _ = fs::create_dir_all(icon.parent().unwrap());
            let _ = fs::copy(src, &icon);
        }
        let entry = format!(
            "[Desktop Entry]\nType=Application\nName={}\nComment={}\nExec=\"{}\" %F\nIcon={}\nTerminal=false\nCategories=Graphics;AudioVideo;Office;\nStartupWMClass={}\n",
            app.name,
            app.tagline,
            i.exe.display(),
            icon.display(),
            if app.wm_class.is_empty() { i.exe.file_name().unwrap().to_string_lossy().into_owned() } else { app.wm_class.clone() }
        );
        let _ = write_atomic(&desktop_file(&app.id), entry.as_bytes());
    }

    #[cfg(target_os = "linux")]
    pub fn remove(app: &Release) {
        let _ = fs::remove_file(desktop_file(&app.id));
    }

    /// The entry of a renamed app under its former id.
    #[cfg(target_os = "linux")]
    pub fn remove_former(old: &Former) {
        let _ = fs::remove_file(desktop_file(&old.id));
        let _ = fs::remove_file(dirs::data_dir().unwrap().join("icons/hicolor/512x512/apps").join(format!("media.rng.studio.{}.png", old.id)));
    }

    #[cfg(windows)]
    fn lnk(name: &str) -> Option<PathBuf> {
        Some(dirs::data_dir()?.join("Microsoft\\Windows\\Start Menu\\Programs\\RNG Media").join(format!("{name}.lnk")))
    }

    #[cfg(windows)]
    pub fn create(app: &Release, i: &Installed) {
        if let Some(l) = lnk(&app.name) {
            let _ = fs::create_dir_all(l.parent().unwrap());
            if let Ok(s) = mslnk::ShellLink::new(&i.exe) {
                let _ = s.create_lnk(l);
            }
        }
    }

    #[cfg(windows)]
    pub fn remove(app: &Release) {
        if let Some(l) = lnk(&app.name) {
            let _ = fs::remove_file(l);
        }
    }

    #[cfg(windows)]
    pub fn remove_former(old: &Former) {
        if let Some(l) = lnk(&old.name).filter(|_| !old.name.is_empty()) {
            let _ = fs::remove_file(l);
        }
    }

    #[cfg(target_os = "macos")]
    pub fn create(_: &Release, _: &Installed) {}
    #[cfg(target_os = "macos")]
    pub fn remove(_: &Release) {}
    #[cfg(target_os = "macos")]
    pub fn remove_former(_: &Former) {}
}

/// Updates RNG Studio itself, then starts the new copy. Returns only on failure.
pub fn self_update(rel: &Release, progress: &dyn Fn(u64, u64)) -> Result<()> {
    let asset = rel.asset().ok_or("no update for this computer")?;
    let work = data_dir().join("self-update");
    let _ = fs::remove_dir_all(&work);
    let archive = download(asset, &work, progress)?;
    let unpacked = work.join("x");
    extract(&archive, &unpacked)?;
    let new = unpacked.join(&asset.exe);
    let me = std::env::current_exe().map_err(|e| e.to_string())?;

    #[cfg(target_os = "macos")]
    {
        // Swap the whole bundle: .../RNG Studio.app/Contents/MacOS/rng-studio. The new bundle takes
        // its own name, so an old "LaunchLlama.app" becomes "RNG Studio.app".
        let bundle = me.ancestors().nth(3).ok_or("can't find the app bundle")?.to_path_buf();
        if bundle.extension().is_some_and(|e| e == "app") {
            let target = bundle.with_file_name(new.file_name().ok_or("bad update")?);
            let old = bundle.with_extension("app.old");
            let _ = fs::remove_dir_all(&old);
            fs::rename(&bundle, &old).map_err(|e| e.to_string())?;
            if let Err(e) = fs::rename(&new, &target) {
                let _ = fs::rename(&old, &bundle);
                return Err(e.to_string());
            }
            let _ = fs::remove_dir_all(&old);
            let _ = std::process::Command::new("open").arg("-n").arg(&target).spawn();
            std::process::exit(0);
        }
        let inner = new.join("Contents/MacOS").join(me.file_name().unwrap());
        self_replace::self_replace(&inner).map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "macos"))]
    self_replace::self_replace(&new).map_err(|e| e.to_string())?;

    let _ = fs::remove_dir_all(&work);
    let _ = std::process::Command::new(&me).spawn();
    std::process::exit(0);
}

/// Linux and Windows: the first run from a download copies RNG Studio into its own folder and
/// adds a menu entry, then runs from there. Returns true when this process should exit.
pub fn self_install() -> bool {
    #[cfg(any(target_os = "linux", windows))]
    {
        if std::env::var_os("RNG_STUDIO_NO_INSTALL").is_some() {
            return false;
        }
        let Ok(me) = std::env::current_exe() else { return false };
        let home = data_dir().join("bin");
        let target = home.join(if cfg!(windows) { "RNG Studio.exe" } else { "rng-studio" });
        if me.starts_with(&home) {
            write_own_shortcut(&me);
            return false;
        }
        if fs::create_dir_all(&home).is_err() {
            return false;
        }
        let tmp = target.with_extension("new");
        if fs::copy(&me, &tmp).is_err() || fs::rename(&tmp, &target).is_err() {
            return false;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&target, fs::Permissions::from_mode(0o755));
        }
        write_own_shortcut(&target);
        return std::process::Command::new(&target).spawn().is_ok();
    }
    #[allow(unreachable_code)]
    false
}

#[cfg(target_os = "linux")]
fn write_own_shortcut(exe: &Path) {
    let share = dirs::data_dir().unwrap();
    let icon = share.join("icons/hicolor/512x512/apps/media.rng.studio.png");
    if fs::read(&icon).ok().as_deref() != Some(crate::ICON_BIG) {
        let _ = fs::create_dir_all(icon.parent().unwrap());
        let _ = fs::write(&icon, crate::ICON_BIG);
    }
    // Pre-rename entry and icon.
    let _ = fs::remove_file(share.join("applications/media.rng.launchllama.desktop"));
    let _ = fs::remove_file(share.join("icons/hicolor/512x512/apps/media.rng.launchllama.png"));
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=RNG Studio\nComment=Install, open and update RNG Media apps\nExec=\"{}\"\nIcon=media.rng.studio\nTerminal=false\nCategories=Utility;\nStartupWMClass=media.rng.studio\n",
        exe.display()
    );
    let path = share.join("applications/media.rng.studio.desktop");
    if fs::read_to_string(&path).ok().as_deref() != Some(entry.as_str()) {
        let _ = write_atomic(&path, entry.as_bytes());
    }
}

#[cfg(windows)]
fn write_own_shortcut(exe: &Path) {
    if let Some(dir) = dirs::data_dir().map(|d| d.join("Microsoft\\Windows\\Start Menu\\Programs\\RNG Media")) {
        let _ = fs::remove_file(dir.join("LaunchLlama.lnk"));
        let l = dir.join("RNG Studio.lnk");
        if !l.exists() {
            let _ = fs::create_dir_all(&dir);
            if let Ok(s) = mslnk::ShellLink::new(exe) {
                let _ = s.create_lnk(l);
            }
        }
    }
}

#[cfg(test)]
mod sig_tests {
    #[test]
    fn signed_manifests() {
        // Output of publish.sh's signing step for the studio-b12 manifest.
        let good = include_str!("../tests/signed-manifest.json");
        assert!(super::parse_manifest(good, false).is_ok());
        let bad = good.replacen("\\\"build\\\":12", "\\\"build\\\":13", 1);
        assert_ne!(good, bad);
        assert!(super::parse_manifest(&bad, false).is_err());
        assert!(super::parse_manifest(r#"{"launcher":null,"apps":[]}"#, false).is_ok());
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn zip_drops_setuid_and_keeps_symlinks() {
        let dir = std::env::temp_dir().join(format!("rng-studio-zip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let zpath = dir.join("t.zip");
        {
            let mut w = zip::ZipWriter::new(std::fs::File::create(&zpath).unwrap());
            let o = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).unix_permissions(0o4755);
            w.start_file("app/run", o).unwrap();
            std::io::Write::write_all(&mut w, b"#!/bin/sh\n").unwrap();
            w.add_symlink("app/link", "run", zip::write::SimpleFileOptions::default()).unwrap();
            w.finish().unwrap();
        }
        let out = dir.join("x");
        super::extract(&zpath, &out).unwrap();
        let mode = std::fs::metadata(out.join("app/run")).unwrap().permissions().mode() & 0o7777;
        assert_eq!(mode, 0o755);
        assert_eq!(std::fs::read_link(out.join("app/link")).unwrap().to_str(), Some("run"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn opens_a_desktop_entry() {
        let dir = std::env::temp_dir().join(format!("rng-studio-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mark = dir.join("opened");
        let entry = dir.join("x.desktop");
        std::fs::write(&entry, format!("[Desktop Entry]\nExec=sh -c \"touch {}\" %U\n", mark.display())).unwrap();
        super::launch_detected(&entry).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(500));
        assert!(mark.exists());
        let _ = std::fs::remove_dir_all(dir);
    }
}
