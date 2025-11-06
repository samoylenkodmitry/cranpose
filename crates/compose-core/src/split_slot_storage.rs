//! Split slot storage backend.
//!
//! The long-term goal for this backend is to separate the structural layout of
//! the slot table from the payload values that are "remembered" across
//! recompositions. The previous experimental implementation attempted to do
//! this with a custom data structure, but it diverged from the behaviour of the
//! reference `SlotTable` and missed several tricky invariants around gap
//! restoration, anchor management, and recomposition bookkeeping. As a result
//! it failed a large portion of the integration test suite once the split
//! backend became the default.
//!
//! To unblock that transition, the current implementation delegates all layout
//! operations to the proven `SlotTable` while keeping the surface type so the
//! backend can evolve independently in the future. This ensures behavioural
//! parity with the baseline backend while still allowing tests to exercise the
//! split backend entry points.

use crate::{
    slot_storage::{GroupId, SlotStorage, StartGroup, ValueSlotId},
    Key, NodeId, Owned, ScopeId, SlotTable,
};

/// Split slot storage backed by the baseline `SlotTable` implementation.
pub struct SplitSlotStorage {
    inner: SlotTable,
}

impl SplitSlotStorage {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Debug helper that mirrors [`SlotTable::debug_dump_groups`].
    #[must_use]
    pub fn debug_dump_groups(&self) -> Vec<(usize, Key, Option<ScopeId>, usize)> {
        self.inner.debug_dump_groups()
    }

    /// Debug helper that mirrors [`SlotTable::debug_dump_all_slots`].
    #[must_use]
    pub fn debug_dump_all_slots(&self) -> Vec<(usize, String)> {
        self.inner.debug_dump_all_slots()
    }
}

impl Default for SplitSlotStorage {
    fn default() -> Self {
        Self {
            inner: SlotTable::new(),
        }
    }
}

impl SlotStorage for SplitSlotStorage {
    type Group = GroupId;
    type ValueSlot = ValueSlotId;

    fn begin_group(&mut self, key: Key) -> StartGroup<Self::Group> {
        <SlotTable as SlotStorage>::begin_group(&mut self.inner, key)
    }

    fn set_group_scope(&mut self, group: Self::Group, scope: ScopeId) {
        <SlotTable as SlotStorage>::set_group_scope(&mut self.inner, group, scope);
    }

    fn end_group(&mut self) {
        <SlotTable as SlotStorage>::end_group(&mut self.inner);
    }

    fn skip_current_group(&mut self) {
        <SlotTable as SlotStorage>::skip_current_group(&mut self.inner);
    }

    fn nodes_in_current_group(&self) -> Vec<NodeId> {
        <SlotTable as SlotStorage>::nodes_in_current_group(&self.inner)
    }

    fn begin_recompose_at_scope(&mut self, scope: ScopeId) -> Option<Self::Group> {
        <SlotTable as SlotStorage>::begin_recompose_at_scope(&mut self.inner, scope)
    }

    fn end_recompose(&mut self) {
        <SlotTable as SlotStorage>::end_recompose(&mut self.inner);
    }

    fn alloc_value_slot<T: 'static>(&mut self, init: impl FnOnce() -> T) -> Self::ValueSlot {
        <SlotTable as SlotStorage>::alloc_value_slot(&mut self.inner, init)
    }

    fn read_value<T: 'static>(&self, slot: Self::ValueSlot) -> &T {
        <SlotTable as SlotStorage>::read_value(&self.inner, slot)
    }

    fn read_value_mut<T: 'static>(&mut self, slot: Self::ValueSlot) -> &mut T {
        <SlotTable as SlotStorage>::read_value_mut(&mut self.inner, slot)
    }

    fn write_value<T: 'static>(&mut self, slot: Self::ValueSlot, value: T) {
        <SlotTable as SlotStorage>::write_value(&mut self.inner, slot, value);
    }

    fn remember<T: 'static>(&mut self, init: impl FnOnce() -> T) -> Owned<T> {
        <SlotTable as SlotStorage>::remember(&mut self.inner, init)
    }

    fn peek_node(&self) -> Option<NodeId> {
        <SlotTable as SlotStorage>::peek_node(&self.inner)
    }

    fn record_node(&mut self, id: NodeId) {
        <SlotTable as SlotStorage>::record_node(&mut self.inner, id);
    }

    fn advance_after_node_read(&mut self) {
        <SlotTable as SlotStorage>::advance_after_node_read(&mut self.inner);
    }

    fn step_back(&mut self) {
        <SlotTable as SlotStorage>::step_back(&mut self.inner);
    }

    fn finalize_current_group(&mut self) -> bool {
        <SlotTable as SlotStorage>::finalize_current_group(&mut self.inner)
    }

    fn reset(&mut self) {
        <SlotTable as SlotStorage>::reset(&mut self.inner);
    }

    fn flush(&mut self) {
        <SlotTable as SlotStorage>::flush(&mut self.inner);
    }
}
