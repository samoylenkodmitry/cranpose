use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use compose_foundation::{ModifierNode, ModifierNodeElement, NodeCapabilities};

/// Unique identifier generator for modifier local keys.
static NEXT_MODIFIER_LOCAL_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ModifierLocalId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ModifierLocalToken {
    id: ModifierLocalId,
    type_id: TypeId,
}

impl ModifierLocalToken {
    fn new(type_id: TypeId) -> Self {
        let id = ModifierLocalId(NEXT_MODIFIER_LOCAL_ID.fetch_add(1, Ordering::Relaxed));
        Self { id, type_id }
    }

    fn id(&self) -> ModifierLocalId {
        self.id
    }

    fn type_id(&self) -> TypeId {
        self.type_id
    }
}

/// Type-safe key referencing a modifier local value.
#[derive(Clone)]
pub struct ModifierLocalKey<T: 'static> {
    token: ModifierLocalToken,
    default: Rc<dyn Fn() -> T>,
}

impl<T: 'static> ModifierLocalKey<T> {
    pub fn new(factory: impl Fn() -> T + 'static) -> Self {
        Self {
            token: ModifierLocalToken::new(TypeId::of::<T>()),
            default: Rc::new(factory),
        }
    }

    pub(crate) fn token(&self) -> ModifierLocalToken {
        self.token
    }

    pub(crate) fn default_value(&self) -> T {
        (self.default)()
    }
}

impl<T: 'static> PartialEq for ModifierLocalKey<T> {
    fn eq(&self, other: &Self) -> bool {
        self.token == other.token
    }
}

impl<T: 'static> Eq for ModifierLocalKey<T> {}

impl<T: 'static> Hash for ModifierLocalKey<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.token.hash(state);
    }
}

/// Creates a new modifier local key with the provided default factory.
pub fn modifier_local_of<T: 'static>(factory: impl Fn() -> T + 'static) -> ModifierLocalKey<T> {
    ModifierLocalKey::new(factory)
}

/// Node responsible for providing a modifier local value.
pub struct ModifierLocalProviderNode {
    token: ModifierLocalToken,
    value_factory: Rc<dyn Fn() -> Box<dyn Any>>,
    value: Rc<dyn Any>,
}

impl ModifierLocalProviderNode {
    fn new(token: ModifierLocalToken, factory: Rc<dyn Fn() -> Box<dyn Any>>) -> Self {
        Self {
            token,
            value: Self::create_value(&factory),
            value_factory: factory,
        }
    }

    fn update_value(&mut self) {
        self.value = Self::create_value(&self.value_factory);
    }

    fn token(&self) -> ModifierLocalToken {
        self.token
    }

    fn value(&self) -> Rc<dyn Any> {
        self.value.clone()
    }

    fn create_value(factory: &Rc<dyn Fn() -> Box<dyn Any>>) -> Rc<dyn Any> {
        Rc::from(factory())
    }
}

impl ModifierNode for ModifierLocalProviderNode {}

/// Node responsible for observing modifier local changes.
pub struct ModifierLocalConsumerNode {
    callback: Rc<dyn for<'a> Fn(&mut ModifierLocalReadScope<'a>)>,
}

impl ModifierLocalConsumerNode {
    fn new(callback: Rc<dyn for<'a> Fn(&mut ModifierLocalReadScope<'a>)>) -> Self {
        Self { callback }
    }

    fn notify(&self, scope: &mut ModifierLocalReadScope<'_>) {
        (self.callback)(scope);
    }
}

impl ModifierNode for ModifierLocalConsumerNode {}

#[derive(Clone)]
pub struct ModifierLocalProviderElement {
    token: ModifierLocalToken,
    factory: Rc<dyn Fn() -> Box<dyn Any>>,
}

impl ModifierLocalProviderElement {
    pub fn new<T, F>(key: ModifierLocalKey<T>, factory: F) -> Self
    where
        T: 'static,
        F: Fn() -> T + 'static,
    {
        let erased = Rc::new(move || -> Box<dyn Any> { Box::new(factory()) });
        Self {
            token: key.token(),
            factory: erased,
        }
    }
}

impl fmt::Debug for ModifierLocalProviderElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModifierLocalProviderElement")
            .field("id", &self.token.id())
            .finish()
    }
}

impl PartialEq for ModifierLocalProviderElement {
    fn eq(&self, other: &Self) -> bool {
        self.token == other.token && Rc::ptr_eq(&self.factory, &other.factory)
    }
}

impl Eq for ModifierLocalProviderElement {}

impl Hash for ModifierLocalProviderElement {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.token.hash(state);
        Rc::as_ptr(&self.factory).hash(state);
    }
}

impl ModifierNodeElement for ModifierLocalProviderElement {
    type Node = ModifierLocalProviderNode;

    fn create(&self) -> Self::Node {
        ModifierLocalProviderNode::new(self.token, self.factory.clone())
    }

    fn update(&self, node: &mut Self::Node) {
        node.update_value();
    }

    fn capabilities(&self) -> NodeCapabilities {
        NodeCapabilities::MODIFIER_LOCALS
    }
}

#[derive(Clone)]
pub struct ModifierLocalConsumerElement {
    callback: Rc<dyn for<'a> Fn(&mut ModifierLocalReadScope<'a>)>,
}

impl ModifierLocalConsumerElement {
    pub fn new<F>(callback: F) -> Self
    where
        F: for<'a> Fn(&mut ModifierLocalReadScope<'a>) + 'static,
    {
        Self {
            callback: Rc::new(callback),
        }
    }
}

impl fmt::Debug for ModifierLocalConsumerElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ModifierLocalConsumerElement")
    }
}

impl PartialEq for ModifierLocalConsumerElement {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.callback, &other.callback)
    }
}

impl Eq for ModifierLocalConsumerElement {}

impl Hash for ModifierLocalConsumerElement {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Rc::as_ptr(&self.callback).hash(state);
    }
}

impl ModifierNodeElement for ModifierLocalConsumerElement {
    type Node = ModifierLocalConsumerNode;

    fn create(&self) -> Self::Node {
        ModifierLocalConsumerNode::new(self.callback.clone())
    }

    fn update(&self, node: &mut Self::Node) {
        node.callback = self.callback.clone();
    }

    fn capabilities(&self) -> NodeCapabilities {
        NodeCapabilities::MODIFIER_LOCALS
    }
}

/// Lightweight read scope surfaced to modifier local consumers.
pub struct ModifierLocalReadScope<'a> {
    providers: &'a HashMap<ModifierLocalId, Rc<dyn Any>>,
    fallbacks: HashMap<ModifierLocalId, Rc<dyn Any>>,
}

impl<'a> ModifierLocalReadScope<'a> {
    fn new(providers: &'a HashMap<ModifierLocalId, Rc<dyn Any>>) -> Self {
        Self {
            providers,
            fallbacks: HashMap::new(),
        }
    }

    pub fn get<T: 'static>(&mut self, key: &ModifierLocalKey<T>) -> &T {
        if let Some(value) = self.providers.get(&key.token().id()) {
            return value
                .downcast_ref::<T>()
                .expect("modifier local type mismatch");
        }

        let value = self
            .fallbacks
            .entry(key.token().id())
            .or_insert_with(|| Rc::new(key.default_value()) as Rc<dyn Any>);
        value
            .downcast_ref::<T>()
            .expect("modifier local default type mismatch")
    }
}

#[derive(Default)]
pub struct ModifierLocalManager;

impl ModifierLocalManager {
    pub fn new() -> Self {
        Self
    }

    pub fn sync(&mut self, chain: &mut compose_foundation::ModifierNodeChain) {
        if !chain.has_capability(NodeCapabilities::MODIFIER_LOCALS) {
            return;
        }
        let mut providers: HashMap<ModifierLocalId, Rc<dyn Any>> = HashMap::new();
        chain.visit_nodes_mut(|node, capabilities| {
            if !capabilities.contains(NodeCapabilities::MODIFIER_LOCALS) {
                return;
            }
            if let Some(provider) = node.as_any().downcast_ref::<ModifierLocalProviderNode>() {
                providers.insert(provider.token().id(), provider.value());
            } else if let Some(consumer) = node
                .as_any_mut()
                .downcast_mut::<ModifierLocalConsumerNode>()
            {
                let mut scope = ModifierLocalReadScope::new(&providers);
                consumer.notify(&mut scope);
            }
        });
    }
}
