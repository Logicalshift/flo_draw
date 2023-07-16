use uuid::*;
use lazy_static::*;

use std::collections::{HashMap};
use std::hash::{Hash, Hasher};
use std::sync::*;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The next local ID to assign (so every new namespace has a unique ID)
static NEXT_LOCAL_ID: AtomicUsize = AtomicUsize::new(0);

lazy_static! {
    static ref KNOWN_IDS: Mutex<HashMap<Uuid, usize>> = Mutex::new(HashMap::new());
}

///
/// Specifies the ID of a namespace
///
/// Other IDs (layer, sprite, texture, gradient IDs) are all associated with a namespace: every namespace has a unique set
/// of each resource type, (ie, `SpriteId(0)` refers to a different resource in different namespaces)
///
/// Namespaces have a local ID that's unique within the current process, and a global ID that can be used to identify
/// the namespace when sharing drawings between multiple processes.
///
/// The main use case for namespaces is for when a rendering target has many clients: a client can use its own namespace
/// to avoid needing to coordinate with other clients over which resources it can use.
///
#[derive(Clone, Copy)]
pub struct NamespaceId {
    /// The local ID of this namespace, which is used to compare the namespace inside the process
    local_id: usize,

    /// The global ID of this namespace, which is used to compare the namespace outside of the process
    global_id: Uuid,
}

impl NamespaceId {
    ///
    /// Creates a new unique namespace ID
    ///
    pub fn new() -> NamespaceId {
        // Generate IDs
        let local_id    = NEXT_LOCAL_ID.fetch_add(1, Ordering::Relaxed);
        let global_id   = Uuid::new_v4();

        // Associate the global ID with the local ID
        KNOWN_IDS.lock().unwrap().insert(global_id, local_id);

        NamespaceId { local_id, global_id }
    }

    ///
    /// Creates a namespace with a known global ID
    ///
    pub fn with_id(global_id: Uuid) -> NamespaceId {
        let mut known_ids = KNOWN_IDS.lock().unwrap();

        if let Some(local_id) = known_ids.get(&global_id).copied() {
            // Seen this ID before, re-use the local ID
            NamespaceId { local_id, global_id }
        } else {
            // New ID
            let local_id = NEXT_LOCAL_ID.fetch_add(1, Ordering::Relaxed);
            known_ids.insert(global_id, local_id);

            NamespaceId { local_id, global_id }
        }
    }

    /// Retrieves the local ID (unique within this process) for this namespace
    pub fn local_id(&self) -> usize { self.local_id }

    /// Retrieves the global ID (globally unique) for this namespace
    pub fn global_id(&self) -> Uuid { self.global_id }
}

impl PartialEq for NamespaceId {
    #[inline]
    fn eq(&self, other: &NamespaceId) -> bool {
        self.local_id == other.local_id
    }
}

impl Eq for NamespaceId { }

impl Hash for NamespaceId {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.local_id.hash(state);
    }
}

impl Default for NamespaceId {
    /// The default namespace is the one that is selected when a canvas is first generated
    fn default() -> Self {
        NamespaceId::with_id(uuid!["00000000-0000-0000-0000-000000000000"])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn create_default() {
        assert!(NamespaceId::default() == NamespaceId::default());
    }

    #[test]
    fn create_new() {
        assert!(NamespaceId::new() != NamespaceId::default());
        assert!(NamespaceId::new() != NamespaceId::new());
    }

    #[test]
    fn create_known_id() {
        assert!(NamespaceId::with_id(uuid!["498D1CE4-D05B-43D0-BBDD-DDBE6F3AF6E7"]) != NamespaceId::default());
        assert!(NamespaceId::with_id(uuid!["498D1CE4-D05B-43D0-BBDD-DDBE6F3AF6E7"]) == NamespaceId::with_id(uuid!["498D1CE4-D05B-43D0-BBDD-DDBE6F3AF6E7"]));
    }
}
