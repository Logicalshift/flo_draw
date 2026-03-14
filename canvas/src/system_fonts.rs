use super::font_spec::*;
use super::font_face::*;

use font_kit::handle::{Handle};
use font_kit::source::{SystemSource};

use std::collections::*;
use std::pin::*;
use std::sync::*;

///
/// Loads a system font (with no caching). Returns the empty set if there's no font matching the spec
///
/// Use `FontCache::default().font(spec)` to load a font from the cache (will re-use the existing font if there is one)
///
pub fn load_system_font_family(spec: impl Into<FontSpec>) -> Vec<CanvasFontFace> {
    let spec = spec.into();

    // Use a font-kit system source
    static SOURCE: LazyLock<SystemSource> = LazyLock::new(|| SystemSource::new());

    // Convert the specification into a font-kit specification
    let (family_names, properties) = spec.into();

    // Ask font-kit to retrieve a handle for this font
    let Some(handle) = SOURCE.select_best_match(&family_names, &properties).ok() else { return vec![] };

    // Load from memory or a file
    // font-kit returns a font index here but at least on Mac OS often fails to provide the cloest match in the family
    let data = match handle {
        Handle::Memory { bytes, .. } => {
            let boxed: Box<[u8]> = (*bytes).clone().into_boxed_slice();
            Arc::new(Pin::new(boxed))
        }

        Handle::Path { path, .. } => {
            let Some(bytes) = std::fs::read(path).ok() else { return vec![] };
            let boxed: Box<[u8]> = bytes.into_boxed_slice();
            Arc::new(Pin::new(boxed))
        }
    };

    // Convert to a font face
    CanvasFontFace::family_from_pinned(data)
}

///
/// Returns the index of the font face that matches the specification
///
fn match_font<'a>(fonts: impl Iterator<Item=&'a CanvasFontFace>, spec: &FontSpec) -> Option<usize> {
    let mut low_score = f64::MAX;
    let mut best_font = None;

    for (idx, font) in fonts.enumerate() {
        let Some(this_font_spec) = font.spec() else { continue };

        let score = spec.score(&this_font_spec);
        if score < low_score {
            low_score = score;
            best_font = Some(idx);
        }
    }

    best_font
}

///
/// Loads a system font matching the given spec
///
pub fn load_system_font(spec: impl Into<FontSpec>) -> Option<CanvasFontFace> {
    let spec = spec.into();

    // Load the family for this font spec
    let mut family = load_system_font_family(&spec);

    // Match the font from the family (font-kit is *supposed* to be able to this for us but this fails on Mac OS at least by not applying weights or styles)
    let font_idx = match_font(family.iter(), &spec)?;

    // Return the value from the family that matched
    Some(family.remove(font_idx))
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
/// Represents a font family that we have loaded (name of the family)
///
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
struct FontFamilyKey(Vec<String>, FontFamily);

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
            // Load a new font (or font family)
            let new_font_family = load_system_font_family(&spec);
            let new_font        = if new_font_family.is_empty() {
                // No fonts could be found
                return None;
            } else if new_font_family.len() == 1 {
                // Not a family, just one font
                let mut new_font_family = new_font_family;
                Arc::new(new_font_family.pop().unwrap())
            } else {
                // Use the best match in the family
                let best_match      = match_font(new_font_family.iter(), &spec)?;
                let new_font_family = new_font_family.into_iter().map(|font| Arc::new(font)).collect::<Vec<_>>();

                new_font_family[best_match].clone()
            };

            // Store using the FontSpec derived from the font (so if the user requests similar fonts, we eventually just use the exact spec for each one)
            let spec = if let Some(exact_spec) = new_font.spec() {
                core.exact_spec.insert(spec.clone(), exact_spec.clone());
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
