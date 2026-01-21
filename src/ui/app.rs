use gpui::{
    Action, App, Application, Bounds, Entity, KeyBinding, Menu, MenuItem, SystemMenuType,
    WindowBounds, WindowOptions, actions, prelude::*, px, size,
};

use crate::ui::style::{
    BASE_WINDOW_HEIGHT, BASE_WINDOW_WIDTH, MIN_SCALE, WINDOW_HEIGHT, WINDOW_WIDTH,
};
use crate::ui::view::TetrisView;
use gpui_tetris::audio::AudioEngine;
use gpui_tetris::game::input::GameAction;
use std::env;
use std::path::{Path, PathBuf};

actions!(
    tetris,
    [
        Quit,
        ToggleFullscreen,
        MoveLeft,
        MoveRight,
        SoftDrop,
        HardDrop,
        RotateCw,
        RotateCcw,
        Hold,
        Pause,
        Restart
    ]
);

pub fn run() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            is_resizable: true,
            window_min_size: Some(size(
                px(BASE_WINDOW_WIDTH * MIN_SCALE),
                px(BASE_WINDOW_HEIGHT * MIN_SCALE),
            )),
            ..Default::default()
        };
        let asset_dir = resolve_sfx_dir(cx);
        let audio_engine = match AudioEngine::new(&asset_dir) {
            Ok(engine) => Some(engine),
            Err(err) => {
                eprintln!("audio disabled: {err}");
                None
            }
        };

        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.bind_keys([
            KeyBinding::new("up", RotateCw, None),
            KeyBinding::new("space", HardDrop, None),
            KeyBinding::new("c", Hold, None),
            KeyBinding::new("p", Pause, None),
            KeyBinding::new("r", Restart, None),
        ]);
        cx.set_menus(vec![
            Menu {
                name: "gpui-tetris".into(),
                items: vec![
                    MenuItem::os_submenu("Services", SystemMenuType::Services),
                    MenuItem::separator(),
                    MenuItem::action("Quit", Quit),
                ],
            },
            Menu {
                name: "View".into(),
                items: vec![MenuItem::action("Toggle Full Screen", ToggleFullscreen)],
            },
        ]);

        let window = cx
            .open_window(options, move |_, cx| {
                let audio = audio_engine.clone();
                cx.new(|cx| TetrisView::new(cx, audio))
            })
            .unwrap();
        let window_handle = window;

        cx.on_action({
            let window = window_handle;
            move |_: &ToggleFullscreen, cx| {
                let target = cx.active_window().unwrap_or_else(|| window.into());
                cx.defer(move |cx| {
                    let _ = target.update(cx, |_, window, _| {
                        window.activate_window();
                        window.toggle_fullscreen();
                    });
                });
            }
        });
        cx.bind_keys([
            KeyBinding::new("cmd-ctrl-f", ToggleFullscreen, None),
            KeyBinding::new("ctrl-cmd-f", ToggleFullscreen, None),
        ]);
        let view = window.update(cx, |_, _, cx| cx.entity()).unwrap();

        register_action::<MoveLeft>(cx, view.clone(), GameAction::MoveLeft);
        register_action::<MoveRight>(cx, view.clone(), GameAction::MoveRight);
        register_action::<SoftDrop>(cx, view.clone(), GameAction::SoftDrop);
        register_action::<HardDrop>(cx, view.clone(), GameAction::HardDrop);
        register_action::<RotateCw>(cx, view.clone(), GameAction::RotateCw);
        register_action::<RotateCcw>(cx, view.clone(), GameAction::RotateCcw);
        register_action::<Hold>(cx, view.clone(), GameAction::Hold);
        register_action::<Pause>(cx, view.clone(), GameAction::Pause);
        register_action::<Restart>(cx, view, GameAction::Restart);

        window
            .update(cx, |view, window, _| {
                window.focus(view.focus_handle());
            })
            .unwrap();
        cx.activate(true);
    })
}

fn resolve_sfx_dir(cx: &App) -> PathBuf {
    if let Ok(dir) = env::var("TETRIS_ASSET_DIR") {
        let path = PathBuf::from(dir);
        if path.exists() {
            return path;
        }
    }

    if let Ok(app_path) = cx.app_path() {
        let bundled = app_path.join("Contents/Resources/assets/sfx");
        if bundled.exists() {
            return bundled;
        }
    }

    Path::new("assets/sfx").to_path_buf()
}

fn register_action<A: Action + 'static>(
    cx: &mut App,
    view: Entity<TetrisView>,
    action: GameAction,
) {
    cx.on_action(move |_: &A, cx| {
        view.update(cx, |view, cx| {
            view.receive_action(action);
            cx.notify();
        });
    });
}
