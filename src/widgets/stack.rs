// Based on libcosmic's Stack but with drag_destinations propagation fixed.

use cosmic::iced::core::event::{self, Event};
use cosmic::iced::core::layout;
use cosmic::iced::core::mouse;
use cosmic::iced::core::overlay;
use cosmic::iced::core::renderer;
use cosmic::iced::core::widget::{Operation, Tree};
use cosmic::iced::core::{
    Clipboard, Element, Layout, Length, Rectangle, Shell, Size, Vector, Widget,
};

pub struct Stack<'a, Message, Theme = cosmic::Theme, Renderer = cosmic::Renderer> {
    width: Length,
    height: Length,
    children: Vec<Element<'a, Message, Theme, Renderer>>,
}

impl<'a, Message, Theme, Renderer> Stack<'a, Message, Theme, Renderer>
where
    Renderer: cosmic::iced::core::Renderer,
{
    pub fn new() -> Self {
        Self {
            width: Length::Shrink,
            height: Length::Shrink,
            children: Vec::new(),
        }
    }

    pub fn with_children(
        children: impl IntoIterator<Item = Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        Self {
            width: Length::Shrink,
            height: Length::Shrink,
            children: children.into_iter().collect(),
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn push(mut self, child: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        self.children.push(child.into());
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Stack<'a, Message, Theme, Renderer>
where
    Renderer: cosmic::iced::core::Renderer,
{
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(&mut self.children);
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);

        if self.children.is_empty() {
            return layout::Node::new(limits.resolve(self.width, self.height, Size::ZERO));
        }

        let base = self.children[0]
            .as_widget()
            .layout(&mut tree.children[0], renderer, &limits);

        let size = limits.resolve(self.width, self.height, base.size());
        let limits = layout::Limits::new(Size::ZERO, size);

        let nodes = std::iter::once(base)
            .chain(
                self.children[1..]
                    .iter()
                    .zip(&mut tree.children[1..])
                    .map(|(layer, tree)| layer.as_widget().layout(tree, renderer, &limits)),
            )
            .collect();

        layout::Node::with_children(size, nodes)
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds(), &mut |operation| {
            self.children
                .iter()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((child, state), layout)| {
                    child
                        .as_widget()
                        .operate(state, layout, renderer, operation);
                });
        });
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        mut cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> event::Status {
        let is_over = cursor.is_over(layout.bounds());

        self.children
            .iter_mut()
            .rev()
            .zip(tree.children.iter_mut().rev())
            .zip(layout.children().rev())
            .map(|((child, state), layout)| {
                let status = child.as_widget_mut().on_event(
                    state,
                    event.clone(),
                    layout,
                    cursor,
                    renderer,
                    clipboard,
                    shell,
                    viewport,
                );

                if is_over && cursor != mouse::Cursor::Unavailable {
                    let interaction = child
                        .as_widget()
                        .mouse_interaction(state, layout, cursor, viewport, renderer);

                    if interaction != mouse::Interaction::None {
                        cursor = mouse::Cursor::Unavailable;
                    }
                }

                status
            })
            .find(|&status| status == event::Status::Captured)
            .unwrap_or(event::Status::Ignored)
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.children
            .iter()
            .rev()
            .zip(tree.children.iter().rev())
            .zip(layout.children().rev())
            .map(|((child, state), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(state, layout, cursor, viewport, renderer)
            })
            .find(|&interaction| interaction != mouse::Interaction::None)
            .unwrap_or_default()
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if let Some(clipped_viewport) = layout.bounds().intersection(viewport) {
            let layers_below = if cursor.is_over(layout.bounds()) {
                self.children
                    .iter()
                    .rev()
                    .zip(tree.children.iter().rev())
                    .zip(layout.children().rev())
                    .position(|((layer, state), layout)| {
                        let interaction = layer
                            .as_widget()
                            .mouse_interaction(state, layout, cursor, viewport, renderer);

                        interaction != mouse::Interaction::None
                    })
                    .map_or(0, |i| self.children.len() - i - 1)
            } else {
                0
            };

            let mut layers = self
                .children
                .iter()
                .zip(&tree.children)
                .zip(layout.children())
                .enumerate();

            let layers_ref = layers.by_ref();

            let mut draw_layer =
                |i, layer: &Element<'a, Message, Theme, Renderer>, state, layout, cursor| {
                    if i > 0 {
                        renderer.with_layer(clipped_viewport, |renderer| {
                            layer.as_widget().draw(
                                state,
                                renderer,
                                theme,
                                style,
                                layout,
                                cursor,
                                &clipped_viewport,
                            );
                        });
                    } else {
                        layer.as_widget().draw(
                            state,
                            renderer,
                            theme,
                            style,
                            layout,
                            cursor,
                            &clipped_viewport,
                        );
                    }
                };

            for (i, ((layer, state), layout)) in layers_ref.take(layers_below) {
                draw_layer(i, layer, state, layout, mouse::Cursor::Unavailable);
            }

            for (i, ((layer, state), layout)) in layers_ref {
                draw_layer(i, layer, state, layout, cursor);
            }
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        overlay::from_children(&mut self.children, tree, layout, renderer, translation)
    }

    fn drag_destinations(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        dnd_rectangles: &mut cosmic::iced::core::clipboard::DndDestinationRectangles,
    ) {
        for ((child, state), layout) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
        {
            child.as_widget().drag_destinations(
                state,
                layout.with_virtual_offset(layout.virtual_offset()),
                renderer,
                dnd_rectangles,
            );
        }
    }
}

impl<'a, Message, Theme, Renderer> From<Stack<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: cosmic::iced::core::Renderer + 'a,
{
    fn from(stack: Stack<'a, Message, Theme, Renderer>) -> Self {
        Self::new(stack)
    }
}

#[macro_export]
macro_rules! stack {
    ($($x:expr),* $(,)?) => {
        $crate::widgets::stack::Stack::with_children(vec![$($x.into()),*])
    };
}
