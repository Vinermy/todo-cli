// ----------------------------------           IMPORTS           ----------------------------------
use std::{
    sync::mpsc,
    fs,
    io,
    thread,
    time::Instant
};

use crossterm::{
    event::{EnableMouseCapture, KeyCode},
    event,
    execute,
    event::Event as CEvent,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen}
};

use tui::{
    Terminal,
    widgets::{Block, Borders, BorderType, Cell, List, ListItem, ListState, Paragraph, Row, Table, Tabs},
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans}
};

use chrono::{
    DateTime,
    Duration,
    Utc
};

use serde::{
    Deserialize,
    Serialize
};

use rand::Rng;
use thiserror::Error;
// ----------------------------------        END OF IMPORTS       ----------------------------------


// ----------------------------------          CONSTANTS          ----------------------------------
const DB_PATH: &str = "./data.json";
const ACTIVE_COLOR: Color = Color::White;
const INACTIVE_COLOR: Color = Color::DarkGray;
const BG_HIGHLIGHT_COLOR: Color = Color::Gray;
const FOCUS_COLOR: Color = Color::LightMagenta;


const DEFAULT_BORDER: BorderType = BorderType::Plain;
const FOCUS_BORDER: BorderType = BorderType::Double;
// ----------------------------------       END OF CONSTANTS      ----------------------------------


// ----------------------------------           STRUCTS           ----------------------------------
#[derive(Serialize, Deserialize, Clone)]
struct Todo {
    id: usize,
    name: String,
    category: String,
    text: String,
    created_at: DateTime<Utc>,
}

impl Todo {
    fn default() -> Todo {
        let temp = Todo {
            id: 0,
            name: "".to_string(),
            category: "".to_string(),
            text: "".to_string(),
            created_at: Default::default(),
        };
        temp
    }
}


struct InputStates { // Holds all the input data
    name: String,
    category: String,
    text: String,
}
// ----------------------------------        END OF STRUCTS       ----------------------------------


// ----------------------------------            ENUMS            ----------------------------------
#[derive(PartialEq, Clone, Copy)]
enum FocusedInput { // Holds the current focused input
    Name,
    Category,
    Text,
    None
}


enum Event<I> {
    Input(I),
    Tick
}


#[derive(Error, Debug)]
pub enum Error {
    #[error("error reading the DB file: {0}")]
    ReadDBError(#[from] io::Error),
    #[error("error parsing the DB file: {0}")]
    ParseDBError(#[from] serde_json::Error),
}


#[derive(Copy, Clone, Debug, PartialEq)]
enum MenuItem { // Holds the menu tabs that can be opened
    Home,
    TODOs,
    Add
}

impl From<MenuItem> for usize {
    fn from(input: MenuItem) -> usize {
        match input {
            MenuItem::Home => 0,
            MenuItem::TODOs => 1,
            MenuItem::Add => 2,
        }
    }
}
// ----------------------------------         END OF ENUMS        ----------------------------------


// ----------------------------------      UI BLOCK FUNCTIONS     ----------------------------------
fn copyright_block<'a>() -> Paragraph<'a> { // Render the fake copyright block
    Paragraph::new("todo-CLI 2023 --- all rights reserved")
        .style(Style::default().fg(FOCUS_COLOR))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(ACTIVE_COLOR))
                .title("Copyright")
                .border_type(DEFAULT_BORDER)
        )
}


fn render_add<'a>(input_states: &InputStates, focused_input: &FocusedInput) // Render the Add tab
                  -> (Paragraph<'a>, Paragraph<'a>, Paragraph<'a>, Paragraph<'a>) {

    // Draw help text
    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_type(DEFAULT_BORDER)
        .style(Style::default().fg(ACTIVE_COLOR))
        .title("Help");

    let help =
        Paragraph::new("Use <tab> to switch between fields, <enter> to submit")
        .block(help_block)
        .style(Style::default().fg(FOCUS_COLOR));

    // Create the blocks
    let text_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(ACTIVE_COLOR))
        .title("Text")
        .border_type(
            if focused_input == &FocusedInput::Text {
                FOCUS_BORDER
            } else {
                DEFAULT_BORDER
            }
        )
        .border_style(Style::default().fg(
            if focused_input == &FocusedInput::Text {
                FOCUS_COLOR
            } else {
                ACTIVE_COLOR
            }
        ));

    let name_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Gray))
        .title("Name")
        .border_type(
            if focused_input == &FocusedInput::Name {
                FOCUS_BORDER
            } else {
                DEFAULT_BORDER
            }
        )
        .border_style(Style::default().fg(
            if focused_input == &FocusedInput::Name {
                FOCUS_COLOR
            } else {
                ACTIVE_COLOR
            }
        ));

    let category_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Gray))
        .title("Category")
        .border_type(
            if focused_input == &FocusedInput::Category {
                FOCUS_BORDER
            } else {
                DEFAULT_BORDER
            }
        )
        .border_style(Style::default().fg(
            if focused_input == &FocusedInput::Category {
                FOCUS_COLOR
            } else {
                ACTIVE_COLOR
            }
        ));

    // Draw the name field
    let name = Paragraph::new("Name for a TODO: ".to_owned() + &input_states.name)
        .block(name_block)
        .style(Style::default().fg(
            if focused_input == &FocusedInput::Name {
                ACTIVE_COLOR
            } else {
                INACTIVE_COLOR
            }
        ));

    // Draw the category field
    let category = Paragraph::new("Category for a TODO: ".to_owned() + &input_states.category)
        .block(category_block)
        .style(Style::default().fg(
            if focused_input == &FocusedInput::Category {
                ACTIVE_COLOR
            } else {
                INACTIVE_COLOR
            }
        ));

    // Draw the Text field
    let text = Paragraph::new("Text for a TODO: ".to_owned() + &input_states.text)
        .block(text_block)
        .style(Style::default().fg(
            if focused_input == &FocusedInput::Text {
                ACTIVE_COLOR
            } else {
                INACTIVE_COLOR
            }
        ));

    (help, name, category, text)
}


fn render_todos<'a>(todo_list_state: &ListState) -> (List<'a>, Table<'a>) { // render TODOs tab

    // Create a block for displaying TODOs
    let todos = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(ACTIVE_COLOR))
        .title("TODOs")
        .border_type(DEFAULT_BORDER);

    // Create a list for navigation between TODOs
    let todo_list = read_db().expect("can fetch todo list");
    let items: Vec<_> = todo_list
        .iter()
        .map(|todo| {
            ListItem::new(Spans::from(vec![Span::styled(
                todo.name.clone(),
                Style::default(),
            )]))
        })
        .collect();

    let selected_todo = match todo_list_state.selected() { // Get the selected to_do
        None => {
            Todo::default()
        }
        Some(selection) => {
            match todo_list.get(selection) {
                None => {Todo::default()}
                Some(todo) => { todo.clone() }
            }
        }
    };

    // Put the list inside the block
    let list = List::new(items).block(todos).highlight_style(
        Style::default()
            .bg(BG_HIGHLIGHT_COLOR)
            .fg(FOCUS_COLOR)
            .add_modifier(Modifier::BOLD),
    );

    // Create a table
    let todo_detail = Table::new(vec![Row::new(vec![
        Cell::from(Span::raw(selected_todo.id.to_string())),
        Cell::from(Span::raw(selected_todo.name)),
        Cell::from(Span::raw(selected_todo.category)),
        Cell::from(Span::raw(selected_todo.text)),
        Cell::from(Span::raw(selected_todo.created_at.to_string())),
    ])])
        .header(Row::new(vec![
            Cell::from(Span::styled(
                "ID",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Name",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Category",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Text",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Created At",
                Style::default().add_modifier(Modifier::BOLD),
            )),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(ACTIVE_COLOR))
                .title("Detail")
                .border_type(DEFAULT_BORDER),
        )
        .widths(&[
            Constraint::Percentage(5),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(30),
            Constraint::Percentage(25),
        ]);

    (list, todo_detail)
}


fn render_home<'a>() -> Paragraph<'a> { // Renders the home page
    let home = Paragraph::new(vec![
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Welcome")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("to")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::styled(
            "todo-CLI",
            Style::default().fg(FOCUS_COLOR),
        )]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw(
            "Press 't' to access TODOs, 'a' to add a new TODO \
            and 'd' to delete the currently selected TODO.")]),
    ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(ACTIVE_COLOR))
                .title("Home")
                .border_type(DEFAULT_BORDER),
        );
    home
}
// ----------------------------------  END OF UI BLOCK FUNCTIONS  ----------------------------------


// ----------------------------------     DB-RELATED FUNCTIONS    ----------------------------------
fn read_db() -> Result<Vec<Todo>, Error> { // Get vector containing all to_dos from the db
    let reading_result = fs::read_to_string(DB_PATH);

    if reading_result.is_err() {
        fs::write(DB_PATH, "[]".to_owned()).expect("Can create a file");
    }
    let parsing_result: Result<Vec<Todo>, _> = match reading_result {
        Ok(contents) => { serde_json::from_str(contents.as_str()) }
        Err(_) => { Ok(Vec::new()) }
    };

    match parsing_result { // Check if the db is empty, return empty vector if so
        Ok(parsed) => {Ok(parsed)}
        Err(_) => {Ok(Vec::new())}
    }
}


fn add_todo_from_input_to_db(input_states: &InputStates)
    -> Result<Vec<Todo>, Error> { // Add to_do to the db
    let mut rng = rand::thread_rng();
    let db_content = fs::read_to_string(DB_PATH)?;
    let mut parsed: Vec<Todo> = serde_json::from_str(&db_content)?;

    let default_todo = Todo {
        id: rng.gen_range(0, 9999999),
        name: input_states.name.to_owned(),
        category: input_states.category.to_uppercase().to_owned(),
        text: input_states.text.to_owned(),
        created_at: Utc::now(),
    };

    parsed.push(default_todo);
    fs::write(DB_PATH, &serde_json::to_vec(&parsed)?)?;

    Ok(parsed)
}


fn remove_todo_at_index(todo_list_state: &mut ListState)
    -> Result<(), Error> { // Remove to_do from db
    if let Some(selected) = todo_list_state.selected() {
        let db_content = fs::read_to_string(DB_PATH)?;
        let mut parsed: Vec<Todo> = serde_json::from_str(&db_content)?;
        parsed.remove(selected);
        fs::write(DB_PATH, &serde_json::to_vec(&parsed)?)?;
        todo_list_state.select(
            if selected >= 1 {
                Some(selected - 1)
            } else {
                None
            }
        );
    }
    Ok(())
}
// ---------------------------------- END OF DB-RELATED FUNCTIONS ----------------------------------


// ----------------------------------           FN MAIN           ----------------------------------
fn main() {
    // Create a Terminal
    enable_raw_mode().expect("");
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).expect("");
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("");
    terminal.clear().expect("Can clear terminal");

    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::milliseconds(200);

    thread::spawn(move || { // Input-capturing thread
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(&Duration::from_std(last_tick.elapsed())
                    .expect("Can convert from std::Duration"))
                .unwrap_or_else(|| Duration::seconds(0));

            if event::poll(timeout.to_std().expect("")).expect("poll works") {
                if let CEvent::Key(key) = event::read().expect("can read events") {
                    tx.send(Event::Input(key)).expect("can send events");
                }
            }

            if last_tick.elapsed() >= tick_rate.to_std().expect("") {
                if let Ok(_) = tx.send(Event::Tick) {
                    last_tick = Instant::now();
                }
            }
        }
    });

    let menu_titles = vec![
        "Home", "TODOs", "Add", "Delete", "Quit"
    ]; // Stores all menu tabs
    let mut active_menu_item = MenuItem::Home;

    let mut todo_list_state = ListState::default(); // Stores the current selected to_do
    todo_list_state.select(Some(0));

    let mut inputs = InputStates {  // Stores current values of all inputs
        name: String::new(),
        category: String::new(),
        text: String::new(),
    };

    let mut focused_input = FocusedInput::None; // Stores the current focused input

    // Main loop
    loop {
        terminal.draw(|rect| {
            let size = rect.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(2),
                        Constraint::Length(3),
                    ]
                        .as_ref(),
                )
                .split(size);

            // Render the fake copyright block
            rect.render_widget(copyright_block(), chunks[2]);

            // Render the top menu
            let menu = menu_titles
                .iter()
                .map(|t| {
                    let (first, rest) = t.split_at(1);
                    Spans::from(vec![
                        Span::styled(
                            first,
                            Style::default()
                                .fg(Color::LightYellow)
                                .add_modifier(Modifier::UNDERLINED),
                        ),
                        Span::styled(rest, Style::default().fg(Color::White)),
                    ])
                })
                .collect();

            let tabs = Tabs::new(menu)
                .select(active_menu_item.into())
                .block(Block::default().title("Menu").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::LightYellow))
                .divider(Span::raw("|"));

            rect.render_widget(tabs, chunks[0]);

            match active_menu_item {
                MenuItem::Home => { // Render the "home" tab
                    rect.render_widget(render_home(), chunks[1])
                }
                MenuItem::TODOs => { // Render the "TODOs" tab
                    let todos_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(20), Constraint::Percentage(80)].as_ref(),
                        )
                        .split(chunks[1]);
                    let (left, right) = render_todos(&todo_list_state);
                    rect.render_stateful_widget(left, todos_chunks[0], &mut todo_list_state);
                    rect.render_widget(right, todos_chunks[1]);
                }
                MenuItem::Add => { // Render the "Add to_do" tab
                    let add_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(
                            [
                                Constraint::Min(3),
                                Constraint::Percentage(20),
                                Constraint::Percentage(20),
                                Constraint::Percentage(60),
                            ].as_ref()
                        ).split(chunks[1]);
                    let (help, name, category, text) =
                        render_add(&inputs, &focused_input);

                    rect.render_widget(help, add_chunks[0]);
                    rect.render_widget(name, add_chunks[1]);
                    rect.render_widget(category, add_chunks[2]);
                    rect.render_widget(text, add_chunks[3]);
                }
            }

        }).expect("Can draw"); // End of the terminal.draw()

        match rx.recv().expect("Input received") {
            Event::Input(event) => match (event.code, focused_input) {
                (KeyCode::Char('q'), FocusedInput::None) => { // Quit
                    disable_raw_mode().expect("");
                    terminal.show_cursor().expect("");
                    break;
                }

                // Switch between the tabs
                (KeyCode::Char('h'), FocusedInput::None) => active_menu_item = MenuItem::Home,
                (KeyCode::Char('t'), FocusedInput::None) => active_menu_item = MenuItem::TODOs,
                (KeyCode::Char('a'), FocusedInput::None) => active_menu_item = MenuItem::Add,

                (KeyCode::Char('d'), FocusedInput::None) => { // Remove selected to_do
                    remove_todo_at_index(&mut todo_list_state).expect("can remove todos");
                }

                (KeyCode::Down, FocusedInput::None) => { // Select the lower to_do in the list
                    if let Some(selected) = todo_list_state.selected() {
                        let amount_pets = read_db().expect("can fetch pet list").len();
                        if selected >= amount_pets - 1 {
                            todo_list_state.select(Some(0));
                        } else {
                            todo_list_state.select(Some(selected + 1));
                        }
                    }
                }
                (KeyCode::Up, FocusedInput::None) => { // Select the higher to_do in the list
                    if let Some(selected) = todo_list_state.selected() {
                        let amount_pets = read_db().expect("can fetch pet list").len();
                        if selected > 0 {
                            todo_list_state.select(Some(selected - 1));
                        } else {
                            todo_list_state.select(Some(amount_pets - 1));
                        }
                    }
                }

                (KeyCode::Tab, _) => { // Cycle the focused field
                    if active_menu_item == MenuItem::Add {
                        match focused_input {
                            FocusedInput::Name => { focused_input = FocusedInput::Category }
                            FocusedInput::Category => { focused_input = FocusedInput::Text }
                            FocusedInput::Text => { focused_input = FocusedInput::Name }
                            FocusedInput::None => { focused_input = FocusedInput::Name }
                        }
                    }
                }

                (KeyCode::Char(_), FocusedInput::None) => {}
                (KeyCode::Backspace, FocusedInput::None) => {}

                // Add character to the corresponding field
                (KeyCode::Char(c), FocusedInput::Name) => {inputs.name.push(c)}
                (KeyCode::Char(c), FocusedInput::Category) => {inputs.category.push(c)}
                (KeyCode::Char(c), FocusedInput::Text) => {inputs.text.push(c)}

                // Remove character from the corresponding field
                (KeyCode::Backspace, FocusedInput::Name) => {inputs.name.pop();}
                (KeyCode::Backspace, FocusedInput::Category) => {inputs.category.pop();}
                (KeyCode::Backspace, FocusedInput::Text) => {inputs.text.pop();}


                (KeyCode::Esc, FocusedInput::None) => {}
                (KeyCode::Esc, _) => {  // Clear the focused input so user can switch to another tab
                    focused_input = FocusedInput::None
                }

                (KeyCode::Enter, _) => { // Save new to_do to the db and clean fields
                    if active_menu_item == MenuItem::Add {
                        add_todo_from_input_to_db(&inputs).expect("Can add TODO");
                        focused_input = FocusedInput::None;
                        inputs = InputStates {
                            name: "".to_string(),
                            category: "".to_string(),
                            text: "".to_string(),
                        }
                    }
                }

                _ => {}
            },
            Event::Tick => {}
        } // End of input match
    } // End of draw loop
}
// ----------------------------------        END OF FN MAIN       ----------------------------------