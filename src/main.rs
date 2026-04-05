use slint::{Model, ModelRc, Timer, TimerMode};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

slint::include_modules!();

struct TimerState {
    total_duration: Duration,
    last_start_time: Option<Instant>,
    running: bool,
}

fn main() -> Result<(), slint::PlatformError> {
    let main_window = MainWindow::new()?;

    let timers = Rc::new(slint::VecModel::from(vec![TimerData {
        name: "New Timer".into(),
        time: "00:00:00".into(),
        running: false,
    }]));
    main_window.set_timers_data(ModelRc::from(timers.clone()));
    main_window.set_selected_timer_index(0);

    let timer_states = Rc::new(RefCell::new(vec![TimerState {
        total_duration: Duration::ZERO,
        last_start_time: None,
        running: false,
    }]));

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
    main_window.on_saveData(move || {
        let mut output = String::new();
        output.push_str("Timer Name,Time\n");
        for i in 0..timers_save.row_count() {
            if let Some(data) = timers_save.row_data(i) {
                output.push_str(&format!("{},{}\n", data.name, data.time));
            }
        }
        if let Err(e) = std::fs::write("timers_save.csv", output) {
            eprintln!("Failed to save timers: {}", e);
        } else {
            println!("Timers saved to timers_save.csv successfully!");
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
