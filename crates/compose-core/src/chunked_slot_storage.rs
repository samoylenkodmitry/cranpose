//! Chunked slot storage backend.
//!
//! NOTE: For correctness, this implementation currently delegates to the
//! baseline [`SlotTable`] while preserving the same external API as the
//! experimental chunked backend. This ensures feature parity with the other
//! backends while the optimized chunked representation is iterated on.

use crate::{
    slot_storage::{GroupId, SlotStorage, StartGroup, ValueSlotId},
    Key, NodeId, Owned, ScopeId, SlotTable,
};

/// Slot storage that delegates to the baseline [`SlotTable`] implementation.
#[derive(Default)]
pub struct ChunkedSlotStorage {
    table: SlotTable,
}

impl ChunkedSlotStorage {
    /// Create a new chunked storage instance.
    pub fn new() -> Self {
        Self {
            table: SlotTable::new(),
        }
    }

    /// Debug helper that mirrors [`SlotTable::debug_dump_groups`].
    pub fn debug_dump_groups(&self) -> Vec<(usize, Key, Option<ScopeId>, usize)> {
        self.table.debug_dump_groups()
    }

    /// Debug helper that mirrors [`SlotTable::debug_dump_all_slots`].
    pub fn debug_dump_all_slots(&self) -> Vec<(usize, String)> {
        self.table.debug_dump_all_slots()
    }
}

impl SlotStorage for ChunkedSlotStorage {
    type Group = GroupId;
    type ValueSlot = ValueSlotId;

    fn begin_group(&mut self, key: Key) -> StartGroup<Self::Group> {
        let idx = SlotTable::start(&mut self.table, key);
        let restored = SlotTable::take_last_start_was_gap(&mut self.table);
        StartGroup {
            group: GroupId::new(idx),
            restored_from_gap: restored,
        }
    }

    fn set_group_scope(&mut self, group: Self::Group, scope: ScopeId) {
        SlotTable::set_group_scope(&mut self.table, group.index(), scope);
    }

    fn end_group(&mut self) {
        SlotTable::end(&mut self.table);
    }

    fn skip_current_group(&mut self) {
        SlotTable::skip_current(&mut self.table);
    }

    fn nodes_in_current_group(&self) -> Vec<NodeId> {
        SlotTable::node_ids_in_current_group(&self.table)
    }

    fn begin_recompose_at_scope(&mut self, scope: ScopeId) -> Option<Self::Group> {
        SlotTable::start_recompose_at_scope(&mut self.table, scope).map(GroupId::new)
    }

    fn end_recompose(&mut self) {
        SlotTable::end_recompose(&mut self.table);
    }

    fn alloc_value_slot<T: 'static>(&mut self, init: impl FnOnce() -> T) -> Self::ValueSlot {
        let idx = SlotTable::use_value_slot(&mut self.table, init);
        ValueSlotId::new(idx)
    }

    fn read_value<T: 'static>(&self, slot: Self::ValueSlot) -> &T {
        SlotTable::read_value(&self.table, slot.index())
    }

    fn read_value_mut<T: 'static>(&mut self, slot: Self::ValueSlot) -> &mut T {
        SlotTable::read_value_mut(&mut self.table, slot.index())
    }

    fn write_value<T: 'static>(&mut self, slot: Self::ValueSlot, value: T) {
        SlotTable::write_value(&mut self.table, slot.index(), value);
    }

    fn remember<T: 'static>(&mut self, init: impl FnOnce() -> T) -> Owned<T> {
        SlotTable::remember(&mut self.table, init)
    }

    fn peek_node(&self) -> Option<NodeId> {
        SlotTable::peek_node(&self.table)
    }

    fn record_node(&mut self, id: NodeId) {
        SlotTable::record_node(&mut self.table, id);
    }

    fn advance_after_node_read(&mut self) {
        SlotTable::advance_after_node_read(&mut self.table);
    }

    fn step_back(&mut self) {
        SlotTable::step_back(&mut self.table);
    }

    fn finalize_current_group(&mut self) -> bool {
        SlotTable::trim_to_cursor(&mut self.table)
    }

    fn reset(&mut self) {
        SlotTable::reset(&mut self.table);
    }

    fn flush(&mut self) {
        SlotTable::flush_anchors_if_dirty(&mut self.table);
    }
}
