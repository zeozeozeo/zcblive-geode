use anyhow::Result;
use kittyaudio::Sound;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Button {
    Jump = 1,
    Left = 2,
    Right = 3,
}

impl Button {
    pub fn from_u8(b: u8) -> Self {
        match b {
            1 => Self::Jump,
            2 => Self::Left,
            3 => Self::Right,
            _ => panic!("invalid button value {b}, expected 1..=3"),
        }
    }

    #[inline]
    pub const fn is_platformer(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Timings {
    pub hard: f64,
    pub regular: f64,
    pub soft: f64,
}

impl Default for Timings {
    fn default() -> Self {
        Self {
            hard: 2.0,
            regular: 0.15,
            soft: 0.025,
            // lower = microclicks
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Pitch {
    pub from: f64,
    pub to: f64,
}

impl Default for Pitch {
    fn default() -> Self {
        Self {
            from: 0.98,
            to: 1.02,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct VolumeSettings {
    pub enabled: bool,
    pub spam_time: f64,
    pub spam_vol_offset_factor: f64,
    pub max_spam_vol_offset: f64,
    pub change_releases_volume: bool,
    pub global_volume: f64,
    pub volume_var: f64,
    pub platformer_volume_factor: f64,
}

impl Default for VolumeSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            spam_time: 0.3,
            spam_vol_offset_factor: 1.3,
            max_spam_vol_offset: 0.6,
            change_releases_volume: false,
            global_volume: 1.0,
            volume_var: 0.2,
            platformer_volume_factor: 1.0,
        }
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub enum ClickType {
    HardClick,
    HardRelease,
    Click,
    Release,
    SoftClick,
    SoftRelease,
    MicroClick,
    MicroRelease,
    #[default]
    None,
}

impl ClickType {
    pub fn from_time(push: bool, time: f64, timings: &Timings) -> Self {
        if time > timings.hard {
            if push {
                Self::HardClick
            } else {
                Self::HardRelease
            }
        } else if time > timings.regular {
            if push {
                Self::Click
            } else {
                Self::Release
            }
        } else if time > timings.soft {
            if push {
                Self::SoftClick
            } else {
                Self::SoftRelease
            }
        } else if push {
            Self::MicroClick
        } else {
            Self::MicroRelease
        }
    }

    pub fn preferred(self) -> [Self; 8] {
        use ClickType::*;

        // HardClick => HardClick, Click, SoftClick, MicroClick, HardRelease, Release, SoftRelease, MicroRelease
        // HardRelease => HardRelease, Release, SoftRelease, MicroRelease, HardClick, Click, SoftClick, MicroClick
        // Click => Click, HardClick, SoftClick, MicroClick, Release, HardRelease, SoftRelease, MicroRelease
        // Release => Release, HardRelease, SoftRelease, MicroRelease, Click, HardClick, SoftClick, MicroClick
        // SoftClick => SoftClick, MicroClick, Click, HardClick, SoftRelease, MicroRelease, Release, HardRelease
        // SoftRelease => SoftRelease, MicroRelease, Release, HardRelease, SoftClick, MicroClick, Click, HardClick
        // MicroClick => MicroClick, SoftClick, Click, HardClick, MicroRelease, SoftRelease, Release, HardRelease
        // MicroRelease => MicroRelease, SoftRelease, Release, HardRelease, MicroClick, SoftClick, Click, HardClick

        match self {
            HardClick => [
                HardClick,
                Click,
                SoftClick,
                MicroClick,
                HardRelease,
                Release,
                SoftRelease,
                MicroRelease,
            ],
            HardRelease => [
                HardRelease,
                Release,
                SoftRelease,
                MicroRelease,
                HardClick,
                Click,
                SoftClick,
                MicroClick,
            ],
            Click => [
                Click,
                HardClick,
                SoftClick,
                MicroClick,
                Release,
                HardRelease,
                SoftRelease,
                MicroRelease,
            ],
            Release => [
                Release,
                HardRelease,
                SoftRelease,
                MicroRelease,
                Click,
                HardClick,
                SoftClick,
                MicroClick,
            ],
            SoftClick => [
                SoftClick,
                MicroClick,
                Click,
                HardClick,
                SoftRelease,
                MicroRelease,
                Release,
                HardRelease,
            ],
            SoftRelease => [
                SoftRelease,
                MicroRelease,
                Release,
                HardRelease,
                SoftClick,
                MicroClick,
                Click,
                HardClick,
            ],
            MicroClick => [
                MicroClick,
                SoftClick,
                Click,
                HardClick,
                MicroRelease,
                SoftRelease,
                Release,
                HardRelease,
            ],
            MicroRelease => [
                MicroRelease,
                SoftRelease,
                Release,
                HardRelease,
                MicroClick,
                SoftClick,
                Click,
                HardClick,
            ],
            None => [None, None, None, None, None, None, None, None],
        }
    }

    #[inline]
    pub const fn is_release(self) -> bool {
        matches!(
            self,
            ClickType::HardRelease
                | ClickType::Release
                | ClickType::SoftRelease
                | ClickType::MicroRelease
        )
    }

    #[inline]
    pub const fn is_click(self) -> bool {
        matches!(
            self,
            ClickType::HardClick | ClickType::Click | ClickType::SoftClick | ClickType::MicroClick
        )
    }
}

#[derive(Clone)]
pub struct SoundWrapper {
    pub sound: Sound,
    // pub pathbuf: PathBuf,
    // fmod_sound: *mut FMOD_SOUND,
}

impl SoundWrapper {
    pub fn from_path(path: &Path) -> Result<Self> {
        // load kittyaudio sound
        let sound = Sound::from_path(path)?;
        Ok(Self {
            sound,
            // pathbuf: path.to_path_buf(),
        })
    }
}

impl std::ops::Deref for SoundWrapper {
    type Target = Sound;

    fn deref(&self) -> &Self::Target {
        &self.sound
    }
}

impl std::ops::DerefMut for SoundWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sound
    }
}

#[derive(Clone, Default)]
pub struct PlayerClicks {
    pub hardclicks: Vec<SoundWrapper>,
    pub hardreleases: Vec<SoundWrapper>,
    pub clicks: Vec<SoundWrapper>,
    pub releases: Vec<SoundWrapper>,
    pub softclicks: Vec<SoundWrapper>,
    pub softreleases: Vec<SoundWrapper>,
    pub microclicks: Vec<SoundWrapper>,
    pub microreleases: Vec<SoundWrapper>,
}

fn read_clicks_in_directory(dir: &Path) -> Vec<SoundWrapper> {
    let Ok(dir) = dir.read_dir() else {
        // log::warn!("can't find directory {dir:?}, skipping");
        return vec![];
    };
    let mut sounds = vec![];
    for entry in dir {
        let path = entry.unwrap().path();
        if path.is_file() {
            let sound = SoundWrapper::from_path(&path);
            if let Ok(sound) = sound {
                sounds.push(sound);
            } else if let Err(e) = sound {
                log::error!("failed to load '{path:?}': {e}");
            }
        }
    }
    sounds
}

impl PlayerClicks {
    fn load_from_subdirs(&mut self, path: &Path) {
        let Ok(dir) = path
            .read_dir()
            .map_err(|e| log::warn!("failed to read directory {path:?}: {e}"))
        else {
            return;
        };
        for entry in dir {
            let Ok(entry) = entry.map_err(|e| log::warn!("error in directory entry: {e}")) else {
                continue;
            };
            self.load_from_dir(&entry.path())
        }
    }

    // parses folders like "softclicks", "soft_clicks", "soft click", "microblablablarelease"
    fn load_from_dir(&mut self, path: &Path) {
        log::debug!("trying to match directory {:?}", path);
        if path.is_file() {
            log::debug!("skipping matching file {:?}", path);
            return;
        }
        let filename: String = path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .chars()
            .filter(|c| c.is_alphabetic())
            .flat_map(|c| c.to_lowercase())
            .collect();
        let patterns = [
            (["hardclick", "hardclicks"], &mut self.hardclicks),
            (["hardrelease", "hardreleases"], &mut self.hardreleases),
            (["click", "clicks"], &mut self.clicks),
            (["release", "releases"], &mut self.releases),
            (["softclick", "softclicks"], &mut self.softclicks),
            (["softrelease", "softreleases"], &mut self.softreleases),
            (["microclick", "microclicks"], &mut self.microclicks),
            (["microrelease", "microreleases"], &mut self.microreleases),
        ];
        let mut matched_any = false;
        for (pats, clicks) in patterns {
            if pats.iter().any(|pat| *pat == filename) {
                log::debug!("directory {path:?} matched patterns {pats:?}");
                matched_any = true;
                *clicks = read_clicks_in_directory(path);
            }
        }
        if !matched_any {
            log::warn!("directory {:?} did not match any pattern", path);
        }
    }

    pub fn num_sounds(&self) -> usize {
        self.hardclicks.len()
            + self.hardreleases.len()
            + self.clicks.len()
            + self.releases.len()
            + self.softclicks.len()
            + self.softreleases.len()
            + self.microclicks.len()
            + self.microreleases.len()
    }

    pub fn random_click(&self, click_type: ClickType) -> Option<&SoundWrapper> {
        macro_rules! rand_click {
            ($arr:expr) => {{
                let len = $arr.len();
                if len == 0 {
                    continue;
                }
                $arr.get(fastrand::usize(..len))
            }};
        }

        let preferred = click_type.preferred();
        for typ in preferred {
            use ClickType::*;

            let click = match typ {
                HardClick => rand_click!(self.hardclicks),
                HardRelease => rand_click!(self.hardreleases),
                Click => rand_click!(self.clicks),
                Release => rand_click!(self.releases),
                SoftClick => rand_click!(self.softclicks),
                SoftRelease => rand_click!(self.softreleases),
                MicroClick => rand_click!(self.microclicks),
                MicroRelease => rand_click!(self.microreleases),
                None => continue,
            };
            if let Some(click) = click {
                return Some(click);
            }
        }
        None
    }

    fn clear(&mut self) {
        self.hardclicks.clear();
        self.hardreleases.clear();
        self.clicks.clear();
        self.releases.clear();
        self.softclicks.clear();
        self.softreleases.clear();
        self.microclicks.clear();
        self.microreleases.clear();
    }
}

#[derive(Default)]
pub struct Clickpack {
    pub player1: PlayerClicks,
    pub player2: PlayerClicks,
    pub left1: PlayerClicks,
    pub right1: PlayerClicks,
    pub left2: PlayerClicks,
    pub right2: PlayerClicks,
    pub noise: Option<SoundWrapper>,
    pub num_sounds: usize,
    pub has_platformer_sounds: bool,
    pub name: String,
    pub path: PathBuf,
}

impl std::ops::Index<usize> for Clickpack {
    type Output = PlayerClicks;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.player1,
            1 => &self.player2,
            2 => &self.left1,
            3 => &self.right1,
            4 => &self.left2,
            5 => &self.right2,
            _ => panic!("invalid index"),
        }
    }
}

impl std::ops::IndexMut<usize> for Clickpack {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.player1,
            1 => &mut self.player2,
            2 => &mut self.left1,
            3 => &mut self.right1,
            4 => &mut self.left2,
            5 => &mut self.right2,
            _ => panic!("invalid index"),
        }
    }
}

const CLICKPACK_DIRNAMES: [&str; 6] = ["player1", "player2", "left1", "left2", "right1", "right2"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LoadClickpackFor {
    #[default]
    All,
    Player1,
    Player2,
    Left1,
    Left2,
    Right1,
    Right2,
}

impl LoadClickpackFor {
    const fn to_index(self) -> usize {
        match self {
            Self::All => 0,
            Self::Player1 => 1,
            Self::Player2 => 2,
            Self::Left1 => 3,
            Self::Left2 => 4,
            Self::Right1 => 5,
            Self::Right2 => 6,
        }
    }
}

fn find_noise_file(dir: &Path) -> Option<PathBuf> {
    let Ok(dir) = dir.read_dir() else {
        return None;
    };
    for entry in dir {
        let path = entry.unwrap().path();
        let filename = path.file_name().unwrap().to_str().unwrap();
        // if it's a noise*, etc file we should try to load it
        if path.is_file()
            && (filename.starts_with("noise")
                || filename.starts_with("whitenoise")
                || filename.starts_with("pcnoise")
                || filename.starts_with("background"))
        {
            return Some(path);
        }
    }
    None
}

impl Clickpack {
    fn load_noise(&mut self, dir: &Path) {
        let Some(path) = find_noise_file(dir) else {
            return;
        };
        // try to load noise
        self.noise = SoundWrapper::from_path(/*self.system*/ &path).ok();
    }

    pub fn load_from_path(
        &mut self,
        clickpack_dir: &Path,
        load_for: LoadClickpackFor,
    ) -> Result<()> {
        log::info!("loading clickpack from path {clickpack_dir:?} for {load_for:?}");
        self.path = clickpack_dir.to_path_buf();
        self.name = clickpack_dir
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        if load_for != LoadClickpackFor::All {
            self.noise = None;
        }

        // this is probably the most confusing code i've ever written
        let mut has_cleared = false;
        for (i, dir) in CLICKPACK_DIRNAMES.iter().enumerate() {
            let sound_idx = if load_for != LoadClickpackFor::All {
                load_for.to_index() - 1 // because All is the first variant
            } else {
                i
            };
            log::info!("sound index {sound_idx}");
            let sounds = &mut self[sound_idx];
            if load_for != LoadClickpackFor::All && !has_cleared {
                log::debug!("clearing sound list for {load_for:?}");
                sounds.clear();
                has_cleared = true;
            }

            let mut path = clickpack_dir.to_path_buf();
            path.push(dir);
            log::debug!("loading from dir {path:?}");

            sounds.load_from_subdirs(&path);
            if load_for != LoadClickpackFor::All && sounds.num_sounds() == 0 {
                log::warn!("directory {dir:?} was not found or has no clicks, assuming there isn't a subdirectory");
                sounds.load_from_subdirs(clickpack_dir);
            }

            // try to load noise from the sound directories
            if self.noise.is_none() {
                self.load_noise(&path);
            }
        }

        if !self.has_clicks() {
            log::warn!("folders {CLICKPACK_DIRNAMES:?} were not found in the clickpack, assuming there is only one player");
            self[0].load_from_subdirs(clickpack_dir);
        }

        // try to load noise from the root clickpack dir
        if self.noise.is_none() {
            self.load_noise(clickpack_dir);
        }

        self.num_sounds = self.num_sounds();
        log::info!(
            "amount of sounds in clickpack {clickpack_dir:?}: {}",
            self.num_sounds
        );
        for mode in [
            ("player1", &self.player1),
            ("player2", &self.player2),
            ("left1", &self.left1),
            ("right1", &self.right1),
            ("left2", &self.left2),
            ("right2", &self.right2),
        ] {
            log::info!("    {}: {} sounds", mode.0, mode.1.num_sounds());
            for sounds in [
                ("hardclicks", &mode.1.hardclicks),
                ("hardreleases", &mode.1.hardreleases),
                ("clicks", &mode.1.clicks),
                ("releases", &mode.1.releases),
                ("softclicks", &mode.1.softclicks),
                ("softreleases", &mode.1.softreleases),
                ("microclicks", &mode.1.microclicks),
                ("microreleases", &mode.1.microreleases),
            ] {
                log::info!(
                    "        {}: {} sounds{}",
                    sounds.0,
                    sounds.1.len(),
                    if sounds.1.len() != 0 { " <<<<<<<" } else { "" }
                );
            }
        }
        self.has_platformer_sounds = self.left1.num_sounds() != 0
            || self.right1.num_sounds() != 0
            || self.left2.num_sounds() != 0
            || self.right2.num_sounds() != 0;

        if self.has_clicks() {
            Ok(())
        } else {
            anyhow::bail!("no clicks found in clickpack, did you select the correct folder?")
        }
    }

    fn has_clicks(&self) -> bool {
        self.player1.num_sounds() != 0
            || self.player2.num_sounds() != 0
            || self.left1.num_sounds() != 0
            || self.right1.num_sounds() != 0
            || self.left2.num_sounds() != 0
            || self.right2.num_sounds() != 0
    }

    pub fn num_sounds(&self) -> usize {
        self.player1.num_sounds()
            + self.player2.num_sounds()
            + self.left1.num_sounds()
            + self.right1.num_sounds()
            + self.left2.num_sounds()
            + self.right2.num_sounds()
    }

    pub fn get_random_click(
        &mut self,
        typ: ClickType,
        player2: bool,
        button: Button,
    ) -> SoundWrapper {
        // try to get a random click/release from the player clicks
        // if it doesn't exist for the wanted player, use the other one (guaranteed to have atleast
        // one click)
        let p1 = &self.player1;
        let p2 = &self.player2;
        let l1 = &self.left1;
        let r1 = &self.right1;
        let l2 = &self.left2;
        let r2 = &self.right2;

        fn get_first_valid_click<'a>(
            sources: &'a [&'a PlayerClicks],
            typ: ClickType,
        ) -> SoundWrapper {
            for source in sources {
                if let Some(click) = source.random_click(typ) {
                    return click.clone();
                }
            }
            // this should never trigger unless we have a clickpack with 0 sounds,
            // which is not going to lead us here
            panic!("no valid clicks found, should be unreachable!");
        }

        match button {
            Button::Jump => {
                if !player2 {
                    get_first_valid_click(&[p1, p2, l1, r1, l2, r2], typ)
                } else {
                    get_first_valid_click(&[p2, p1, l2, r2, l1, r1], typ)
                }
            }
            Button::Left => {
                if !player2 {
                    get_first_valid_click(&[l1, r1, p1, l2, r2, p2], typ)
                } else {
                    get_first_valid_click(&[l2, r2, p2, l1, r1, p1], typ)
                }
            }
            Button::Right => {
                if !player2 {
                    get_first_valid_click(&[r1, l1, p1, r2, l2, p2], typ)
                } else {
                    get_first_valid_click(&[r2, l2, p2, r1, l1, p1], typ)
                }
            }
        }
    }

    #[inline]
    pub const fn has_noise(&self) -> bool {
        self.noise.is_some()
    }
}
