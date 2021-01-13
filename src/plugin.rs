//! Contains the plugin and its helper types.
//!
//! The `ShapePlugin`, used at its fullest, provides the creation of shapes with
//! minimal boilerplate.
//!
//! ## How it works
//! When the user calls the [`ShapeSprite::draw`] method from a system in the
//! `UPDATE` stage, it will return a `(ShapeDescriptor, )` type, a single
//! element tuple that gets feeded to Bevy's `Commands::spawn` method as a
//! bundle.
//!
//! Then, in the [`SHAPE`](shape_plugin_stage::SHAPE) stage, there is a system
//! that for each entity containing `ShapeDescriptor`, it inserts the
//! `SpriteBundle` components into the entity and then removes the
//! `ShapeDescriptor` component.

// TODO: Show use of the alternative drawing function.

use crate::{build_mesh, Buffers, VertexConstructor};
use bevy::{
    app::{stage, AppBuilder, Plugin},
    asset::{Assets, Handle},
    ecs::{Commands, Entity, IntoSystem, Query, ResMut, SystemStage},
    math::Vec2,
    prelude::SpriteBundle,
    render::mesh::Mesh,
    sprite::{ColorMaterial, Sprite},
    transform::components::Transform,
};
use lyon_tessellation::{
    path::Path, BuffersBuilder, FillOptions, FillTessellator, StrokeOptions, StrokeTessellator,
};

/// Stages for this plugin.
pub mod shape_plugin_stage {
    /// The stage where the [`ShapeDescriptor`](super::ShapeDescriptor)s are
    /// replaced with `SpriteBundles`.
    pub const SHAPE: &str = "shape";
}

/// Determines if a shape must be filled or stroked.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TessellationMode {
    Fill(FillOptions),
    Stroke(StrokeOptions),
}

/// A couple of `lyon` fill and stroke tessellators.
pub struct Tessellator {
    pub fill: FillTessellator,
    pub stroke: StrokeTessellator,
}

impl Tessellator {
    /// Creates a new `Tessellator` data structure, containing the two types of
    /// Lyon tessellator.
    pub fn new() -> Self {
        Self {
            fill: FillTessellator::new(),
            stroke: StrokeTessellator::new(),
        }
    }
}

/// A plugin that provides resources and a system to draw shapes in Bevy with
/// less boilerplate.
pub struct ShapePlugin;

impl Plugin for ShapePlugin {
    fn build(&self, app: &mut AppBuilder) {
        let tessellator = Tessellator::new();
        app.add_resource(tessellator)
            .add_stage_after(
                stage::UPDATE,
                shape_plugin_stage::SHAPE,
                SystemStage::parallel(),
            )
            .add_system_to_stage(shape_plugin_stage::SHAPE, shapesprite_maker.system());
    }
}

/// An intermediate representation that contains all the data to create a
/// `SpriteBundle` with a custom mesh.
pub struct ShapeDescriptor {
    pub shape: Box<dyn ShapeSprite + Send + Sync>,
    pub material: Handle<ColorMaterial>,
    pub mode: TessellationMode,
    pub transform: Transform,
}

/// A bevy system. Queries all the [`ShapeDescriptor`]s to create a
/// `SpriteBundle` for each one, before deleting them.
fn shapesprite_maker(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut tessellator: ResMut<Tessellator>,
    query: Query<(Entity, &ShapeDescriptor)>,
) {
    for (entity, shape_descriptor) in query.iter() {
        let path = shape_descriptor.shape.generate_path();

        let mut buffers = Buffers::new();

        match shape_descriptor.mode {
            TessellationMode::Fill(ref options) => {
                tessellator
                    .fill
                    .tessellate_path(
                        &path,
                        options,
                        &mut BuffersBuilder::new(&mut buffers, VertexConstructor),
                    )
                    .unwrap();
            }
            TessellationMode::Stroke(ref options) => {
                tessellator
                    .stroke
                    .tessellate_path(
                        &path,
                        options,
                        &mut BuffersBuilder::new(&mut buffers, VertexConstructor),
                    )
                    .unwrap();
            }
        }

        let sprite_bundle = SpriteBundle {
            material: shape_descriptor.material.clone(),
            mesh: meshes.add(build_mesh(&buffers)),
            sprite: Sprite {
                size: Vec2::new(1.0, 1.0),
                ..Default::default()
            },
            transform: shape_descriptor.transform,
            ..Default::default()
        };

        commands.insert(entity, sprite_bundle);
        commands.remove_one::<ShapeDescriptor>(entity);
    }
}

/// Shape structs that implement this trait can be transformed into a
/// [`SpriteBundle`](bevy::sprite::entity::SpriteBundle). See the
/// [`shapes`](crate::shapes) module for some examples.
///
/// # Implementation example
///
/// ```
/// use bevy_prototype_lyon::plugin::ShapeSprite;
/// use lyon_tessellation::{
///     math::{Point, Rect, Size},
///     path::{path::Builder, traits::PathBuilder, Path, Winding},
/// };
///
/// // First, create a struct to hold the shape features:
/// #[derive(Debug, Clone, Copy, PartialEq)]
/// pub struct Rectangle {
///     pub width: f32,
///     pub height: f32,
/// }
///
/// // Implementing the `Default` trait is not required, but it may facilitate the
/// // definition of the shape before spawning it.
/// impl Default for Rectangle {
///     fn default() -> Self {
///         Self {
///             width: 1.0,
///             height: 1.0,
///         }
///     }
/// }
///
/// // Finally, implement the `generate_path` method.
/// impl ShapeSprite for Rectangle {
///     fn generate_path(&self) -> Path {
///         let mut path_builder = Builder::new();
///         path_builder.add_rectangle(
///             &Rect::new(Point::zero(), Size::new(self.width, self.height)),
///             Winding::Positive,
///         );
///         path_builder.build()
///     }
/// }
/// ```
pub trait ShapeSprite {
    /// Generates a Lyon `Path` for the shape.
    fn generate_path(&self) -> Path;

    /// Returns a [`ShapeDescriptor`] entity for the
    /// shape. If spawned into the [`World`](bevy::ecs::World) during the
    /// [`UPDATE`](bevy::app::stage::UPDATE) stage, it will be replaced by a
    /// custom [`SpriteBundle`](bevy::sprite::entity::SpriteBundle)
    /// corresponding to the shape.
    fn draw(
        &self,
        material: Handle<ColorMaterial>,
        mode: TessellationMode,
        transform: Transform,
    ) -> (ShapeDescriptor,)
    where
        Self: Sync + Send + Sized + Clone + 'static,
    {
        let desc = ShapeDescriptor {
            shape: Box::new(self.clone()),
            material: material.clone(),
            mode,
            transform,
        };

        (desc,)
    }
}
