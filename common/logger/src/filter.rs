use tracing::{Subscriber, Event, Metadata};
use tracing::span;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::{Context, Layered};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::any::TypeId;
use tracing::level_filters::LevelFilter;
use tracing::subscriber::Interest;
use std::marker::PhantomData;

/// A Layer that wraps another Layer and allows
/// disabling logs based on a filter function
pub struct FilteredLayer<S, L, F>
where
    S: Subscriber,
    L: Layer<S>,
    F: 'static + Fn(&Metadata, &Context<'_, S>) -> bool,
{
    layer: L,
    filter: F,
    phantom_s: PhantomData<S>,
}

impl<S: Subscriber, L: Layer<S>, F: 'static + Fn(&Metadata, &Context<'_, S>) -> bool>  FilteredLayer<S, L, F> {
    pub fn new(layer: L, filter: F) -> Self {
        Self {
            layer,
            filter,
            phantom_s: PhantomData
        }
    }
}

impl<S: Subscriber, L: Layer<S>, F: 'static + Fn(&Metadata, &Context<'_, S>) -> bool> Layer<S> for FilteredLayer<S, L, F> {

    fn on_event(&self, event: &Event, context: Context<S>) {
        if (self.filter)(&event.metadata(), &context){
            self.layer.on_event(event, context);
        }
    }

    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        self.layer.register_callsite(metadata)
    }

    fn enabled(&self, metadata: &Metadata<'_>, ctx: Context<'_, S>) -> bool {
        self.layer.enabled(metadata, ctx)
    }

    fn new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        self.layer.new_span(attrs, id, ctx)
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        self.layer.max_level_hint()
    }

    fn on_record(&self, span: &span::Id, values: &span::Record<'_>, ctx: Context<'_, S>) {
        self.layer.on_record(span, values, ctx)
    }

    fn on_follows_from(&self, span: &span::Id, follows: &span::Id, ctx: Context<'_, S>) {
        self.layer.on_follows_from(span, follows, ctx)
    }

    fn on_enter(&self, id: &span::Id, ctx: Context<'_, S>) {
        self.layer.on_enter(id, ctx)
    }

    fn on_exit(&self, id: &span::Id, ctx: Context<'_, S>) {
        self.layer.on_exit(id, ctx)
    }

    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        self.layer.on_close(id, ctx)
    }

    fn on_id_change(&self, old: &span::Id, new: &span::Id, ctx: Context<'_, S>) {
        self.layer.on_id_change(old, new, ctx)
    }

}