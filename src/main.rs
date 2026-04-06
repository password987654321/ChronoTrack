use slint::{Model, ModelRc, Timer, TimerMode};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use std::path::PathBuf;

use chrono::Local;
use serde_json::json;

fn ensure_parent_dir_exists(path: &std::path::Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn save_timer_names_to_config(timers: &slint::VecModel<TimerData>) {
    let path = match config_timers_path() {
        Some(p) => p,
        None => return,
    };
    let _ = ensure_parent_dir_exists(&path);

    let names: Vec<String> = (0..timers.row_count())
        .filter_map(|i| timers.row_data(i).map(|d| d.name.to_string()))
        .collect();

    let json_value = json!({ "timers": names });
    if let Err(e) = std::fs::write(&path, json_value.to_string()) {
        eprintln!("Failed to save timers.json: {}", e);
    }
}

fn save_timer_names_to_config_by_rc(timers: &Rc<slint::VecModel<TimerData>>) {
    save_timer_names_to_config(timers.as_ref());
}

slint::include_modules!();

struct TimerState {
    total_duration: Duration,
    last_start_time: Option<Instant>,
    running: bool,
}

fn config_timers_path() -> Option<PathBuf> {
    // ~/.config/chronotrack/timers.json
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config/chronotrack/timers.json"))
}

fn open_save_dialog(default_file_name: &str) -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .set_file_name(default_file_name)
        .save_file()
}

fn default_save_file_name() -> String {
    Local::now().format("%Y-%m-%d.csv").to_string()
}

fn load_timer_names_from_config() -> Vec<String> {
    let path = match config_timers_path() {
        Some(p) => p,
        None => return vec![],
    };

    let Ok(contents) = std::fs::read_to_string(&path) else {
        return vec![];
    };

    // Expected shapes supported:
    // 1) { "timers": ["name1", "name2"] }
    // 2) ["name1", "name2"]
    // (Any other shape yields an empty list.)
    let json: serde_json::Value = match serde_json::from_str(&contents) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    if let Some(arr) = json.get("timers").and_then(|v| v.as_array()) {
        return arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect::<Vec<String>>();
    }

    if let Some(arr) = json.as_array() {
        return arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect::<Vec<String>>();
    }

    vec![]
}

fn main() -> Result<(), slint::PlatformError> {
    let main_window = MainWindow::new()?;

    let loaded_names = load_timer_names_from_config();
    let initial_timers: Vec<TimerData> = if loaded_names.is_empty() {
        vec![TimerData {
            name: "New Timer".into(),
            time: "00:00:00".into(),
            running: false,
        }]
    } else {
        loaded_names
            .into_iter()
            .map(|name| TimerData {
                name: name.into(),
                time: "00:00:00".into(),
                running: false,
            })
            .collect()
    };

    let timers = Rc::new(slint::VecModel::from(initial_timers));
    main_window.set_timers_data(ModelRc::from(timers.clone()));
    main_window.set_selected_timer_index(0);

    let timer_states = Rc::new(RefCell::new(
        (0..timers.row_count())
            .map(|_| TimerState {
                total_duration: Duration::ZERO,
                last_start_time: None,
                running: false,
            })
            .collect::<Vec<TimerState>>(),
    ));

    let timers_add = timers.clone();
    let states_add = timer_states.clone();
    let window_weak = main_window.as_weak();
    main_window.on_addTimer(move || {
        timers_add.push(TimerData {
            name: "New Timer".into(),
            time: "00:00:00".into(),
            running: false,
        });
        states_add.borrow_mut().push(TimerState {
            total_duration: Duration::ZERO,
            last_start_time: None,
            running: false,
        });
        if let Some(w) = window_weak.upgrade() {
            w.set_selected_timer_index((timers_add.row_count() - 1) as i32);
        }
    });

    let timers_remove = timers.clone();
    let states_remove = timer_states.clone();
    let window_weak = main_window.as_weak();
    main_window.on_removeTimer(move |idx| {
        let idx = idx as usize;
        if idx < timers_remove.row_count() {
            timers_remove.remove(idx);
            states_remove.borrow_mut().remove(idx);

            // Persist removal.
            save_timer_names_to_config_by_rc(&timers_remove);

            if let Some(w) = window_weak.upgrade() {
                w.set_selected_timer_index(-1);
            }
        }
    });

    let timers_toggle = timers.clone();
    let states_toggle = timer_states.clone();
    main_window.on_toggleTimer(move |idx| {
        let idx = idx as usize;
        if idx < timers_toggle.row_count() {
            let mut states = states_toggle.borrow_mut();
            let state = &mut states[idx];
            let mut data = timers_toggle.row_data(idx).unwrap();

            if state.running {
                // Stop it
                state.running = false;
                state.total_duration += state.last_start_time.unwrap().elapsed();
                state.last_start_time = None;
                data.running = false;
            } else {
                // Start it
                state.running = true;
                state.last_start_time = Some(Instant::now());
                data.running = true;
            }
            timers_toggle.set_row_data(idx, data);
        }
    });

    let timers_update = timers.clone();
    main_window.on_updateName(move |idx, name| {
        let idx = idx as usize;
        if idx < timers_update.row_count() {
            let mut data = timers_update.row_data(idx).unwrap();
            data.name = name;
            timers_update.set_row_data(idx, data);

            // Persist renamed timer(s) immediately.
            save_timer_names_to_config_by_rc(&timers_update);
        }
    });

    let timers_reset = timers.clone();
    let states_reset = timer_states.clone();
    main_window.on_resetTimer(move |idx| {
        let idx = idx as usize;
        if idx < timers_reset.row_count() {
            let mut states = states_reset.borrow_mut();
            states[idx].running = false;
            states[idx].total_duration = Duration::ZERO;
            states[idx].last_start_time = None;

            let mut data = timers_reset.row_data(idx).unwrap();
            data.running = false;
            data.time = "00:00:00".into();
            timers_reset.set_row_data(idx, data);
        }
    });

    let timers_save = timers.clone();
    main_window.on_saveDataAs(move || {
        let mut output = String::new();
        output.push_str("Timer Name,Time\n");
        for i in 0..timers_save.row_count() {
            if let Some(data) = timers_save.row_data(i) {
                output.push_str(&format!("{},{}\n", data.name, data.time));
            }
        }

        if let Some(path) = open_save_dialog(&default_save_file_name()) {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::write(&path, output) {
                eprintln!("Failed to save timers: {}", e);
            }
        }
    });

    let timer_ui = Rc::new(Timer::default());
    let timers_tick = timers.clone();
    let states_tick = timer_states.clone();
    timer_ui.start(TimerMode::Repeated, Duration::from_millis(100), move || {
        let states = states_tick.borrow_mut();
        for i in 0..timers_tick.row_count() {
            if states[i].running {
                let mut data = timers_tick.row_data(i).unwrap();
                let elapsed =
                    states[i].total_duration + states[i].last_start_time.unwrap().elapsed();
                let hours = elapsed.as_secs() / 3600;
                let minutes = (elapsed.as_secs() % 3600) / 60;
                let seconds = elapsed.as_secs() % 60;
                data.time = format!("{:02}:{:02}:{:02}", hours, minutes, seconds).into();
                timers_tick.set_row_data(i, data);
            }
        }
    });

    main_window.run()
}
