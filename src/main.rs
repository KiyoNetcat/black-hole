pub mod black_hole;
pub mod minimize;

use black_hole::BlackHole;
use glam::Quat;
use minimize::{MinimizeButton, MinimizeButtonEvent};
use stardust_xr_fusion::{
	client::Client,
	interfaces::SpatialRefProxy,
	objects::{connect_client, object_registry::ObjectRegistry, SpatialRefProxyExt},
	project_local_resources,
	root::{RootAspect, RootEvent},
	spatial::{SpatialRef, Transform},
	ClientHandle,
};
use stardust_xr_molecules::tracked::TrackedProxy;
use std::{
	f32::consts::{FRAC_PI_2, PI},
	sync::{mpsc, Arc},
	time::Duration,
};
use tokio::time::sleep;
use tokio_stream::StreamExt;
use zbus::{names::WellKnownName, Connection};

#[tokio::main(flavor = "current_thread")]
async fn main() {
	let client = Client::connect()
		.await
		.expect("Unable to connect to server");
	client
		.setup_resources(&[&project_local_resources!("res")])
		.unwrap();
	let conn = Connection::session().await.unwrap();
	let client_handle = client.handle();
	let async_loop = client.async_event_loop();
	let dbus_connection = connect_client().await.unwrap();
	let object_registry = ObjectRegistry::new(&dbus_connection).await;

	let mut black_hole = BlackHole::new(client_handle.get_root(), object_registry)
		.await
		.unwrap();
	let mut buttons: [Option<MinimizeButton>; 2] = [None, None];
	let mut was_spawned = false;
	if let Some((anchor, offset, tracked)) = controller_transform(&client_handle, &conn).await {
		was_spawned = true;
		let (button, tx) = MinimizeButton::new(&anchor, offset).unwrap();
		update_tracked_state(tracked, tx);
		buttons[0] = Some(button);
	};
	if let Some((anchor, offset, tracked)) = hand_transform(&client_handle, &conn).await {
		was_spawned = true;
		let (button, tx) = MinimizeButton::new(&anchor, offset).unwrap();
		update_tracked_state(tracked, tx);
		buttons[1] = Some(button);
	}
	if !was_spawned {
		println!("hitting the fallback! :3 ?");
		buttons[0] = Some(
			MinimizeButton::new(
				client_handle.get_root(),
				Transform::from_translation([0.0, 0.0, -0.3]),
			)
			.unwrap()
			.0,
		);
	};
	let mut client = async_loop.stop().await.unwrap();
	let loop_future = client.sync_event_loop(|client, _| {
		while let Some(event) = client.get_root().recv_root_event() {
			match event {
				RootEvent::Frame { info } => {
					black_hole.frame(&info);
					for button in buttons.iter_mut().filter_map(Option::as_mut) {
						button.frame(&mut black_hole);
					}
				}
				RootEvent::SaveState { response: _ } => {}
				RootEvent::Ping { response } => {
					response.send_ok(());
				}
			}
		}
	});
	tokio::select! {
		_ = loop_future => {},
		_ = tokio::signal::ctrl_c() => {}
	};
	drop(black_hole);
	_ = client.try_flush().await;
	sleep(Duration::from_millis(50)).await;
}

fn update_tracked_state(tracked: TrackedProxy<'static>, tx: mpsc::Sender<MinimizeButtonEvent>) {
	tokio::spawn(async move {
		if let Ok(is_tracked) = tracked.is_tracked().await {
			_ = tx.send(MinimizeButtonEvent::SetEnabled(is_tracked));
		}
		let mut stream = tracked.receive_is_tracked_changed().await;
		while let Some(value) = stream.next().await {
			if let Ok(is_tracked) = value.get().await {
				_ = tx.send(MinimizeButtonEvent::SetEnabled(is_tracked));
			}
		}
	});
}

pub async fn controller_transform(
	client: &Arc<ClientHandle>,
	conn: &Connection,
) -> Option<(SpatialRef, Transform, TrackedProxy<'static>)> {
	let anchor = SpatialRefProxy::new(
		conn,
		WellKnownName::from_static_str("org.stardustxr.Controllers").ok()?,
		"/org/stardustxr/Controller/left",
	)
	.await
	.ok()?
	.import(client)
	.await?;
	let tracked = TrackedProxy::new(
		conn,
		WellKnownName::from_static_str("org.stardustxr.Controllers").ok()?,
		"/org/stardustxr/Controller/left",
	)
	.await
	.ok()?;

	Some((
		anchor,
		Transform::from_translation_rotation(
			[0.0, 0.01, 0.02],
			Quat::from_rotation_x(PI + FRAC_PI_2),
		),
		tracked,
	))
}
pub async fn hand_transform(
	client: &Arc<ClientHandle>,
	conn: &Connection,
) -> Option<(SpatialRef, Transform, TrackedProxy<'static>)> {
	let anchor = stardust_xr_fusion::objects::interfaces::SpatialRefProxy::new(
		conn,
		WellKnownName::from_static_str("org.stardustxr.Hands").ok()?,
		"/org/stardustxr/Hand/left/palm",
	)
	.await
	.ok()?
	.import(client)
	.await?;
	let tracked = TrackedProxy::new(
		conn,
		WellKnownName::from_static_str("org.stardustxr.Hands").ok()?,
		"/org/stardustxr/Hand/left",
	)
	.await
	.ok()?;

	Some((
		anchor,
		Transform::from_translation_rotation([0.0, 0.03, 0.0], Quat::from_rotation_x(-FRAC_PI_2)),
		tracked,
	))
}
