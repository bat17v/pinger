use std::{fs, collections::HashMap, cell::RefCell, rc::Rc};

//use rand::Rng;
use chrono::{Local, Timelike};

use gtk::glib;
use gtk::glib::Propagation;
use gtk::{prelude::*, CssProvider};
use gtk::{Application, ApplicationWindow, Box, Orientation, Label, Button};

use gtk4_layer_shell::{Layer, LayerShell, KeyboardMode};


fn get_now() -> i32 {
    let now = Local::now();
    return (now.hour() * 60 + now.minute()) as i32;
}

fn main() {
    let app = Application::builder()
        .application_id("ru.pinger.daemon")
        .build();
    app.connect_activate(build_ui);
    app.run();
}

#[derive(Clone)]
struct AppWidgets {
    window: ApplicationWindow,
    label: Label,
    ans1: Button,
    ans2: Button,
    statuses: Rc<RefCell<HashMap<String, TaskStatus>>>,
    current_task_code: Rc<RefCell<Option<String>>>, 
}

#[derive(Debug)]
struct Task {
    start: Option<i32>,
    duration: Option<i32>,
    title: String,
    code: String,
}

#[derive(Clone, Default, Debug)]
struct TaskStatus {
    started: bool,
    skip_count: i32
}

fn parse_time(t: &str) -> i32 {
    let parts: Vec<&str> = t.split(':').collect();
    match parts.len() {
        1 => parts[0].parse::<i32>().expect("Expected i32 in time!"),
        2 => {
            let h = parts[0].parse::<i32>().expect("Expected i32 in time!");
            let m = parts[1].parse::<i32>().expect("Expected i32 in time!");
            h * 60 + m
        }
        _ => panic!("You can use only mm or hh:mm")
    }
}

fn read_config(path: &str) -> Vec<Task> {
    //let mut rng = rand::thread_rng();
    fs::read_to_string(path)
        .expect("Cannot read config!")
        .lines().map(String::from)
        .filter(|s| !s.trim().is_empty())
        .map(|s| {
            let mut start = None;
            let mut duration = None;
            let mut title_parts = Vec::new();
            let mut code_opt = None;
            
            for p in s.split(' ') {
                if p.is_empty() {continue;}
                
                if p.starts_with('$') {
                    start = Some(parse_time(&p[1..]));
                } else if p.starts_with('~') {
                    duration = Some(parse_time(&p[1..]));
                } else if p.starts_with('@') {
                    code_opt = Some(p[1..].to_string());
                } else {
                    title_parts.push(p);
                }
            }

            //let code = code_opt.unwrap_or_else(|| rng.gen_range(1000..99999).to_string());
            let code = code_opt.unwrap_or(start.unwrap().to_string());

            Task{start, duration, title: title_parts.join(" "), code}
        }).collect()
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("pinger")
        .default_width(400)
        .default_height(200)
        .build();
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::OnDemand);

    let provider = CssProvider::new();
    provider.load_from_path("src/style.css");
    gtk::style_context_add_provider_for_display(
        &gtk::prelude::WidgetExt::display(&window),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION
    );

    window.add_css_class("window");
    
    let ask_box = Box::new(Orientation::Vertical, 10);
    let ask_label = Label::new(Some("Писать pinger"));
    ask_label.add_css_class("title");
    ask_box.append(&ask_label);

    let buttons_box = Box::new(Orientation::Horizontal, 30);
    buttons_box.set_margin_top(20);
    buttons_box.set_halign(gtk::Align::Center);

    let ans1_button = Button::with_label("Делаю");
    let ans2_button = Button::with_label("Закрыть");
    ans1_button.add_css_class("ans-btn");
    ans2_button.add_css_class("ans-btn");
    buttons_box.append(&ans1_button);
    buttons_box.append(&ans2_button);

    let info_label = Label::new(Some("Skips: 4\nStep: Пишу напоминалку"));
    info_label.set_justify(gtk::Justification::Center);
    info_label.set_margin_top(15);
    ask_box.append(&info_label);
    ask_box.append(&buttons_box);

    window.set_child(Some(&ask_box));

    let window_clone = window.clone();
    ans2_button.connect_clicked(move |_| {
        window_clone.set_visible(false);
    });

    let statuses = Rc::new(RefCell::new(HashMap::<String, TaskStatus>::new()));
    let current_task_code = Rc::new(RefCell::new(None::<String>));

    let controller = gtk::EventControllerKey::new();
    let ans1_clone = ans1_button.clone();
    let ans2_clone = ans2_button.clone();

    controller.connect_key_pressed(move |_, _, keycode, _| {
        match keycode {
            41 => {
                ans1_clone.emit_clicked(); 
            }
            44 => {
                ans2_clone.emit_clicked();
            }
            _ => {}
        }
        Propagation::Proceed
    });
    window.add_controller(controller);

    let widgets = AppWidgets {
        window: window.clone(),
        label: ask_label.clone(),
        ans1: ans1_button.clone(),
        ans2: ans2_button.clone(),
        statuses: statuses.clone(),
        current_task_code: current_task_code.clone(),
    };

    let statuses_clone = statuses.clone();
    let current_code_clone = current_task_code.clone();
    let window_clone = window.clone();

    ans1_button.connect_clicked(move |_| {
        if let Some(ref code) = *current_code_clone.borrow() {
            let mut status_map = statuses_clone.borrow_mut();
            let task_status = status_map.entry(code.clone()).or_insert_with(TaskStatus::default);
            task_status.started = true;
            window_clone.set_visible(false);
        }
    });

    run_pinger_loop(widgets);
}

fn run_pinger_loop(widgets: AppWidgets) {
    glib::spawn_future_local(async move {
        loop {
            glib::timeout_future(std::time::Duration::from_secs(5)).await;
            let tasks = read_config("local/test.txt");
            
            for task in tasks {
                let is_started = {
                    let mut status_map = widgets.statuses.borrow_mut();
                    let task_status = status_map.entry(task.code.clone()).or_insert_with(TaskStatus::default);
                    task_status.started
                };
                if task.start.unwrap_or(0) <= get_now() && !is_started {
                    *widgets.current_task_code.borrow_mut() = Some(task.code.clone());
                    widgets.label.set_text(&task.title);
                    widgets.window.present();
                    break;
                }
            }
        }
    });
}
