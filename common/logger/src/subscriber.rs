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

/*
pub struct MyFilter<S>{
    pub sub: S,
    pub env_filter: EnvFilter
}

impl<S: Subscriber, L: Layer<S>> Layer<S> for MyFilter<L> {
    fn on_event(&self, event: &Event, context: Context<S>) {
        if self.env_filter.enabled(event.metadata(), context.clone()) {
            self.sub.on_event(event, context);
        }
    }
}

*/

pub struct ToggleFilter<S>{
    pub sub: S,
    pub enabled: Arc<AtomicBool>
}

impl<S: Subscriber, L: Layer<S>> Layer<S> for ToggleFilter<L> {

    fn on_event(&self, event: &Event, context: Context<S>) {
        if self.enabled.load(Ordering::Relaxed) {
            self.sub.on_event(event, context);
        }
    }
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        self.sub.register_callsite(metadata)
    }

    fn enabled(&self, metadata: &Metadata<'_>, ctx: Context<'_, S>) -> bool {
        self.sub.enabled(metadata, ctx)
    }

    fn new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        self.sub.new_span(attrs, id, ctx)
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        self.sub.max_level_hint()
    }

    fn on_record(&self, span: &span::Id, values: &span::Record<'_>, ctx: Context<'_, S>) {
        self.sub.on_record(span, values, ctx)
    }

    fn on_follows_from(&self, span: &span::Id, follows: &span::Id, ctx: Context<'_, S>) {
        self.sub.on_follows_from(span, follows, ctx)
    }

    fn on_enter(&self, id: &span::Id, ctx: Context<'_, S>) {
        self.sub.on_enter(id, ctx)
    }

    fn on_exit(&self, id: &span::Id, ctx: Context<'_, S>) {
        self.sub.on_exit(id, ctx)
    }

    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        self.sub.on_close(id, ctx)
    }

    fn on_id_change(&self, old: &span::Id, new: &span::Id, ctx: Context<'_, S>) {
        self.sub.on_id_change(old, new, ctx)
    }

}

pub struct FilteredLayer<S, L, F>
where
    S: Subscriber,
    L: Layer<S>,
    F: 'static + Fn(&Metadata, &Context<'_, S>) -> bool,
{
    pub sub: L,
    pub filter: F,
    pub phantom_s: PhantomData<S>,
}

impl<S: Subscriber, L: Layer<S>, F: 'static + Fn(&Metadata, &Context<'_, S>) -> bool,> Layer<S> for FilteredLayer<S, L, F> {

    fn on_event(&self, event: &Event, context: Context<S>) {
        if (self.filter)(&event.metadata(), &context){
            self.sub.on_event(event, context);
        }
    }

    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        self.sub.register_callsite(metadata)
    }

    fn enabled(&self, metadata: &Metadata<'_>, ctx: Context<'_, S>) -> bool {
        self.sub.enabled(metadata, ctx)
    }

    fn new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        self.sub.new_span(attrs, id, ctx)
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        self.sub.max_level_hint()
    }

    fn on_record(&self, span: &span::Id, values: &span::Record<'_>, ctx: Context<'_, S>) {
        self.sub.on_record(span, values, ctx)
    }

    fn on_follows_from(&self, span: &span::Id, follows: &span::Id, ctx: Context<'_, S>) {
        self.sub.on_follows_from(span, follows, ctx)
    }

    fn on_enter(&self, id: &span::Id, ctx: Context<'_, S>) {
        self.sub.on_enter(id, ctx)
    }

    fn on_exit(&self, id: &span::Id, ctx: Context<'_, S>) {
        self.sub.on_exit(id, ctx)
    }

    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        self.sub.on_close(id, ctx)
    }

    fn on_id_change(&self, old: &span::Id, new: &span::Id, ctx: Context<'_, S>) {
        self.sub.on_id_change(old, new, ctx)
    }

}