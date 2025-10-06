use rustc_hash::FxHashMap;
use stardust_xr_fusion::{
	drawable::Model,
	node::{NodeResult, NodeType},
	objects::{
		interfaces::{ReparentLockProxy, ReparentableProxy},
		object_registry::ObjectRegistry,
		ObjectInfo,
	},
	query::{ObjectQuery, QueryEvent},
	root::FrameInfo,
	spatial::{Spatial, SpatialAspect, SpatialRefAspect, Transform},
	values::ResourceID,
};
use std::sync::Arc;
use tween::{ExpoIn, ExpoOut, Tweener};

pub enum AnimationState {
	Idle,
	Expand(Tweener<f32, f32, ExpoOut>),
	Contract(Tweener<f32, f32, ExpoIn>),
}

type Query = (
	ReparentableProxy<'static>,
	Option<ReparentLockProxy<'static>>,
);
pub struct BlackHole {
	pub spatial: Spatial,
	spatial_id: u64,
	query: ObjectQuery<Query, ()>,
	_visuals: Model,
	open: bool,
	animation_state: AnimationState,
	reparentable: FxHashMap<ObjectInfo, Query>,
	captured: FxHashMap<ObjectInfo, Query>,
}
impl BlackHole {
	pub async fn new(
		spatial_parent: &impl SpatialRefAspect,
		object_registry: Arc<ObjectRegistry>,
	) -> NodeResult<BlackHole> {
		let spatial = Spatial::create(spatial_parent, Transform::identity(), false)?;
		let spatial_id = spatial.export_spatial().await?;
		let query = ObjectQuery::new(object_registry, ());

		let _visuals = Model::create(
			&spatial,
			Transform::from_scale([10.0; 3]),
			&ResourceID::new_namespaced("black_hole", "black_hole"),
		)?;

		spatial.set_local_transform(Transform::from_scale([0.0001; 3]))?;

		Ok(BlackHole {
			spatial,
			spatial_id,
			query,
			_visuals,
			open: true,
			animation_state: AnimationState::Idle,
			reparentable: FxHashMap::default(),
			captured: FxHashMap::default(),
		})
	}
	pub fn open(&self) -> bool {
		self.open
	}
	pub fn in_transition(&self) -> bool {
		!matches!(&self.animation_state, AnimationState::Idle)
	}
	pub fn frame(&mut self, info: &FrameInfo) {
		while let Ok(event) = self.query.try_recv_event() {
			match event {
				QueryEvent::NewMatch(object_info, reparentable) => {
					self.reparentable.insert(object_info, reparentable);
				}
				QueryEvent::MatchModified(object_info, reparentable) => {
					self.reparentable.insert(object_info, reparentable);
				}
				QueryEvent::MatchLost(object_info) => {
					self.reparentable.remove(&object_info);
				}
				QueryEvent::PhantomVariant(_) => (),
			}
		}
		match &mut self.animation_state {
			AnimationState::Expand(e) => {
				let _ = self._visuals.set_enabled(true);
				let scale = e.move_by(info.delta);

				// Apply scale to the spatial transform
				let _ = self
					.spatial
					.set_local_transform(Transform::from_scale([scale.max(0.0001); 3]));

				if e.is_finished() {
					self.animation_state =
						AnimationState::Contract(Tweener::expo_in_at(1.0, 0.0, 0.25, 0.0));

					if self.open {
						// Opening: release captured objects back to their original parents
						for (_, (reparentable, locked)) in self.captured.drain() {
							if let Some(locked) = locked {
								tokio::spawn(async move {
									_ = locked.unlock().await;
									_ = reparentable.unparent().await;
								});
							} else {
								tokio::spawn(async move {
									_ = reparentable.unparent().await;
								});
							}
						}
					} else {
						// Closing: capture all available reparentable objects
						for (object_info, (reparentable, locked)) in self.reparentable.iter() {
							let reparentable = reparentable.clone();
							let locked = locked.clone();
							let spatial_id = self.spatial_id;

							self.captured.insert(
								object_info.clone(),
								(reparentable.clone(), locked.clone()),
							);

							tokio::spawn(async move {
								if let Some(locked) = &locked {
									_ = locked.lock().await;
								}
								_ = reparentable.parent(spatial_id).await;
							});
						}
					}
				}
			}
			AnimationState::Contract(c) => {
				let scale = c.move_by(info.delta);

				// Apply scale to the spatial transform
				let _ = self
					.spatial
					.set_local_transform(Transform::from_scale([scale.max(0.0001); 3]));

				if c.is_finished() {
					let _ = self._visuals.set_enabled(false);
					self.animation_state = AnimationState::Idle;
				}
			}
			_ => (),
		};
	}
	pub fn toggle(&mut self) {
		self.open = !self.open;
		self.animation_state = AnimationState::Expand(Tweener::expo_out_at(0.0, 1.0, 0.25, 0.0));
	}
}
impl Drop for BlackHole {
	fn drop(&mut self) {
		// Reset spatial scale
		let _ = self
			.spatial
			.set_local_transform(Transform::from_scale([1.0; 3]));

		// Release all captured objects
		for (reparentable, locked) in self.captured.values() {
			let reparentable = reparentable.clone();
			let locked = locked.clone();

			tokio::spawn(async move {
				if let Some(locked) = locked {
					_ = locked.unlock().await;
				}
				_ = reparentable.unparent().await;
			});
		}
	}
}
