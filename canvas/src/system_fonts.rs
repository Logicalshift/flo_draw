use super::font_spec::*;
use super::font_face::*;

use font_kit::handle::{Handle};
use font_kit::source::{SystemSource};

use std::collections::*;
use std::pin::*;
use std::sync::*;

///
/// Loads a system font (with no caching)
///
/// Use `FontCache::default().font(spec)` to load a font from the cache (will re-use the existing font if there is one)
///
pub fn load_system_font(spec: impl Into<FontSpec>) -> Option<CanvasFontFace> {
    let spec = spec.into();

    // Use a font-kit system source
    static SOURCE: LazyLock<SystemSource> = LazyLock::new(|| SystemSource::new());

    // Convert the specification into a font-kit specification
    let (family_names, properties) = spec.into();

    // Ask font-kit to retrieve a handle for this font
    let handle = SOURCE.select_best_match(&family_names, &properties).ok()?;

    // Load from memory or a file
    let (data, font_index) = match handle {
        Handle::Memory { bytes, font_index } => {
            let boxed: Box<[u8]> = (*bytes).clone().into_boxed_slice();
            (Arc::new(Pin::new(boxed)), font_index)
        }

        Handle::Path { path, font_index } => {
            let bytes = std::fs::read(path).ok()?;
            let boxed: Box<[u8]> = bytes.into_boxed_slice();
            (Arc::new(Pin::new(boxed)), font_index)
        }
    };

    // Convert to a font face
    Some(CanvasFontFace::from_pinned(data, font_index))
}

///
/// Loads a font from the standard cache (shortcut for `FontCache::default().font(spec))
///
#[inline]
pub fn font(spec: impl Into<FontSpec>) -> Option<Arc<CanvasFontFace>> {
    FontCache::default().font(spec)
}

struct FontCacheCore {
    /// Strong refs, enqueued in usage order
    strong_ref_queue: VecDeque<FontSpec>,

    /// Returns the exact FontSpec of the fonts we've loaded (from the spec that loaded them) 
    exact_spec: HashMap<FontSpec, FontSpec>,

    /// The fonts that are being kept as strong refs
    strong_refs: HashMap<FontSpec, Arc<CanvasFontFace>>,

    /// Weak font refs, so we can re-use fonts that are still referenced elsewhere in the app
    weak_refs: HashMap<FontSpec, Weak<CanvasFontFace>>,

    /// The maximum number of weak refs to keep around before trimming them (grows dynamically if we keep a lot of fonts in memory)
    max_weak_refs: usize,
}

///
/// Cache that can be used to avoid reloading fonts that are already in use
///
pub struct FontCache {
    /// The number of fonts that are kept in the cache with a hard reference (so they won't expire even when nothing is using them)
    max_kept: usize,

    /// The font cache core, protected by a mutex so multiple threads can share a font cache
    core: Mutex<FontCacheCore>,
}

impl FontCache {
    ///
    /// Creates a new font cache
    ///
    /// In general, use the shared font cache from `FontCache::default()` unless you want to have independently loaded fonts for
    /// some part of your application.
    ///
    pub fn new() -> Self {
        let core = FontCacheCore {
            strong_ref_queue:   VecDeque::new(),
            exact_spec:         HashMap::new(),
            strong_refs:        HashMap::new(),
            weak_refs:          HashMap::new(),
            max_weak_refs:      8,
        };

        FontCache {
            max_kept:   3,
            core:       Mutex::new(core),
        }
    }

    ///
    /// Changes the maximum number of fonts that should be kept as strong references in the cache
    ///
    pub fn with_max_kept(mut self, new_max_kept: usize) -> Self {
        self.max_kept = new_max_kept;

        self
    }

    ///
    /// Returns the shared font cache (there's one of these per app, so this shares fonts between everything)
    ///
    #[inline]
    pub fn default<'a>() -> &'a FontCache {
        // The shared font cache
        static SHARED_FONT_CACHE: LazyLock<FontCache> = LazyLock::new(|| FontCache::new().with_max_kept(8));

        &SHARED_FONT_CACHE
    }

    ///
    /// Retrieves or loads the font with a particular specification
    ///
    pub fn font(&self, spec: impl Into<FontSpec>) -> Option<Arc<CanvasFontFace>> {
        let spec        = spec.into();
        let mut core    = self.core.lock().unwrap();

        // If we've loaded a font using this spec before, we might have an 'exact' spec that will save us reloading an existing font
        let spec = if let Some(exact_spec) = core.exact_spec.get(&spec) { exact_spec.clone() } else { spec };

        if let Some(strong_ref) = core.strong_refs.get(&spec).cloned() {
            // Put the ref to the back of the queue to be removed (it will already be in the queue if it's in the strong_refs table)
            core.strong_ref_queue.retain(|val| val != &spec);
            core.strong_ref_queue.push_back(spec.clone());

            // Use the strong ref
            Some(strong_ref)
        } else if let Some(weak_ref) = core.weak_refs.get(&spec).and_then(|font| font.upgrade()) {
            // Promote a weak ref
            core.strong_refs.insert(spec.clone(), weak_ref.clone());
            core.strong_ref_queue.push_back(spec.clone());
            core.trim_strong_refs(self.max_kept);

            Some(weak_ref)
        } else {
            // Load a new font
            let new_font = load_system_font(&spec)?;
            let new_font = Arc::new(new_font);

            // Store using the FontSpec derived from the font (so if the user requests similar fonts, we eventually just use the exact spec for each one)
            let spec = if let Some(exact_spec) = new_font.spec() {
                core.exact_spec.insert(exact_spec.clone(), spec.clone());
                exact_spec
            } else {
                spec
            };

            // Store as a strong ref
            core.strong_refs.insert(spec.clone(), new_font.clone());
            core.strong_ref_queue.push_back(spec.clone());
            core.trim_strong_refs(self.max_kept);
            core.trim_weak_refs();

            Some(new_font)
        }
    }
}

impl FontCacheCore {
    ///
    /// Moves any strong refs to the weak refs until only 'max_kept' remain
    ///
    fn trim_strong_refs(&mut self, max_kept: usize) {
        while self.strong_refs.len() > max_kept {
            // Fetch the next font to downgrade
            let Some(to_remove) = self.strong_ref_queue.pop_front() else { break; };
            let Some(font)      = self.strong_refs.remove(&to_remove) else { continue; };

            // Downgrade to the weak queue
            let font = Arc::downgrade(&font);
            self.weak_refs.insert(to_remove, font);
        }
    }

    ///
    /// Removes any weak references to fonts that are no longer in use
    ///
    fn trim_weak_refs(&mut self) {
        // Do nothing if the weak_refs hash table isn't large enough yet
        if self.weak_refs.len() < self.max_weak_refs {
            return;
        }

        // Remove any weak refs that are no longer in use
        self.weak_refs.retain(|_, weak_ref| weak_ref.upgrade().is_some());

        // Grow our max size exponentially if there are still too many weak refs
        while self.max_weak_refs < self.weak_refs.len() {
            self.max_weak_refs *= 2;
        }
    }
}
