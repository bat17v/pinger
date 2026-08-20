use std::fs::metadata;
use std::{fs, collections::HashMap, cell::RefCell, rc::Rc};

//use rand::Rng;
use chrono::{DateTime, Local, Utc};

use gtk::glib;
use gtk::glib::Propagation;
use gtk::{prelude::*, CssProvider};
use gtk::{Application, ApplicationWindow, Box, Orientation, Label, Button};

use gtk4_layer_shell::{Layer, LayerShell, KeyboardMode};


fn get_now() -> i32 {
    let now = Local::now();
    return (now.timestamp() / 60) as i32;
}

fn minutes_to_day_start(s: i32) -> i32 {
    return s - s % (24 * 60);
}

fn main() {
    let app = Application::builder()
        .application_id("ru.pinger.daemon")
        .build();
    let controller = Rc::new(AppController::default());
    let controller_clone = controller.clone();
    app.connect_activate(move |app| {
        build_ui(&app, &controller_clone);
    });
    app.run();
}


#[derive(Clone, Debug)]
struct Task {
    start: Option<i32>,
    duration: Option<i32>,
    restart: Option<i32>,
    title: String,
    code: String,
}

#[derive(Clone, Default, Debug)]
struct TaskStatus {
    started: bool,
    skip_count: i32,
    real_start: Option<i32>
}

pub enum Action {
    ShowWindow { title: String },
    HideWindow,
    DoNothing,
}

#[derive(Default)]
pub struct AppController {
    statuses: Rc<RefCell<HashMap<String, TaskStatus>>>,
    current_task: Rc<RefCell<Option<Task>>>
}

impl AppController {
    pub fn click1(&self) -> Action {
        if let Some(ref task) = *self.current_task.borrow() {
            let mut status_map = self.statuses.borrow_mut();
            let task_status = status_map.entry(task.code.clone()).or_insert_with(TaskStatus::default);
            task_status.started = true;
        }
        return Action::HideWindow;
    }
    pub fn click2(&self) -> Action {
        if let Some(ref task) = *self.current_task.borrow() {
            let mut status_map = self.statuses.borrow_mut();
            let task_status = status_map.entry(task.code.clone()).or_insert_with(TaskStatus::default);
            task_status.real_start = Some(get_now() + task.restart.unwrap_or(2));
        }
        return Action::HideWindow;
    }
    pub fn keypress(&self, keycode: u32) -> Action {
        match keycode {
            41 => self.click1(),
            44 => self.click2(),
            _ => Action::DoNothing,
        }
    }
    pub fn regular(&self) -> Action {
        let config_path;
        if let Ok(home) = std::env::var("HOME") {
            config_path = format!("{}/.config/pinger.conf", home);
        } else {
            panic!("Set HOME env variable!")
        }
        let tasks = read_config(&config_path);
       
        let mut t_end = 0;
        for task in tasks {
            let mut status_map = self.statuses.borrow_mut();
            let task_status = status_map.entry(task.code.clone()).or_insert_with(TaskStatus::default);
            
            let start = task_status.real_start.unwrap_or(task.start.unwrap_or(0)).max(t_end);
            t_end = start + task.duration.unwrap_or(20);
            let now = get_now();
            println!("start: {start}, end: {t_end}, now: {now}");
            if start <= now && now <= t_end && !task_status.started {
                let mut current_task_mut = self.current_task.borrow_mut();
                *current_task_mut = Some(task.clone());
                return Action::ShowWindow{title: task.title};
            }
        }
        return Action::DoNothing;
    }
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
    let day_start = minutes_to_day_start(
        if let Ok(metadata) = fs::metadata(path) {
            if let Ok(modif) = metadata.modified() {
                (DateTime::<Utc>::from(modif).with_timezone(&Local).timestamp() / 60) as i32
            } else {
                get_now()
            }
        } else {
            get_now()
        });

    fs::read_to_string(path)
        .expect("Cannot read config!")
        .lines().map(String::from)
        .filter(|s| !s.trim().is_empty())
        .map(|s| {
            let mut start = None;
            let mut duration = None;
            let mut restart = None;
            let mut title_parts = Vec::new();
            let mut code_opt = None;
            
            for p in s.split(' ') {
                if p.is_empty() {continue;}
                
                if p.starts_with('$') {
                    start = Some(day_start + parse_time(&p[1..]));
                } else if p.starts_with('~') {
                    duration = Some(parse_time(&p[1..]));
                } else if p.starts_with('*') {
                    restart = Some(parse_time(&p[1..]));
                } else if p.starts_with('@') {
                    code_opt = Some(p[1..].to_string());
                } else {
                    title_parts.push(p);
                }
            }

            //let code = code_opt.unwrap_or_else(|| rng.gen_range(1000..99999).to_string());
            let code = code_opt.unwrap_or(start.unwrap().to_string());

            Task{start, duration, restart, title: title_parts.join(" "), code}
        }).collect()
}

fn build_ui(gtk_app: &Application, app_controller: &Rc<AppController>) {
    let window = ApplicationWindow::builder()
        .application(gtk_app)
        .title("pinger")
        .default_width(400)
        .default_height(200)
        .build();
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::OnDemand);

    let provider = CssProvider::new();
    provider.load_from_data(include_str!("style.css"));
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

    let info_label = Label::new(None);
    info_label.set_justify(gtk::Justification::Center);
    info_label.set_margin_top(15);
    ask_box.append(&info_label);
    ask_box.append(&buttons_box);

    window.set_child(Some(&ask_box));


    let execute = glib::clone!(
        #[weak] window, #[weak] ask_label,
        move |action: Action| {
            match action {
                Action::ShowWindow { title } => {
                    ask_label.set_text(&title);
                    window.set_visible(true);
                }
                Action::HideWindow => {
                    window.set_visible(false);
                }
                _ => {}
            }
        }
    );

    let controller = gtk::EventControllerKey::new(); 
    let execute_clone = execute.clone();
    controller.connect_key_pressed(glib::clone!(#[to_owned] app_controller,
    move |_, _, keycode, _| -> Propagation {
        execute_clone(app_controller.keypress(keycode));
        return Propagation::Proceed;
    }));
    window.add_controller(controller);

    let execute_clone = execute.clone();
    ans1_button.connect_clicked(glib::clone!(#[to_owned] app_controller,
    move |_| {
        execute_clone(app_controller.click1());
    }));
    
    let execute_clone = execute.clone();
    ans2_button.connect_clicked(glib::clone!(#[to_owned] app_controller,
    move |_| {
        execute_clone(app_controller.click2());
    }));
    
    let execute_clone = execute.clone();
    glib::spawn_future_local(glib::clone!(#[to_owned] app_controller, async move {
        loop {
            execute_clone(app_controller.regular());
            glib::timeout_future(std::time::Duration::from_secs(5)).await;
        }
    }));
}
