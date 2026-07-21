use serde::{Deserialize, Serialize};

use super::{MemoryId, ObjectType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryObjectRef {
    pub object_type: ObjectType,
    pub id: MemoryId,
}

impl MemoryObjectRef {
    pub const fn new(object_type: ObjectType, id: MemoryId) -> Self {
        Self { object_type, id }
    }

    pub(crate) const fn from_id_type(id: MemoryId, object_type: ObjectType) -> Self {
        Self { object_type, id }
    }

    pub(crate) const fn stable_order_key(self) -> (MemoryId, u8) {
        (self.id, self.object_type.stable_rank())
    }
}
