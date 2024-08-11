use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use clap::{Parser, Subcommand};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    Active,
    Done,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Active => write!(f, "on"),
            TaskStatus::Done => write!(f, "done"),
        }
    }
}

impl FromStr for TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "on" | "active" | "a" => Ok(TaskStatus::Active),
            "done" | "d" => Ok(TaskStatus::Done),
            _ => Err(format!("Invalid status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Category(String);

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Category {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Category(s.to_string()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub title: String,
    pub description: String,
    pub creation_date: DateTime<Local>,
    pub category: Category,
    pub status: TaskStatus,
}

impl Task {
    pub fn new(title: String, description: String, category: Category) -> Self {
        Task {
            title,
            description,
            creation_date: Local::now(),
            category,
            status: TaskStatus::Active,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TodoList {
    tasks: HashMap<String, Task>,
    file_path: PathBuf,
}

impl TodoList {
    pub fn new(file_path: PathBuf) -> Self {
        let tasks = if file_path.exists() {
            let content = fs::read_to_string(&file_path).expect("Failed to read file");
            serde_json::from_str(&content).unwrap_or_else(|_| HashMap::new())
        } else {
            HashMap::new()
        };
        TodoList { tasks, file_path }
    }

    pub fn add_task(&mut self, task: Task) -> Result<(), String> {
        if self.tasks.contains_key(&task.title) {
            Err(format!("Task with title '{}' already exists", task.title))
        } else {
            self.tasks.insert(task.title.clone(), task);
            self.save();
            Ok(())
        }
    }

    pub fn mark_as_done(&mut self, title: &str) -> Result<(), String> {
        if let Some(task) = self.tasks.get_mut(title) {
            task.status = TaskStatus::Done;
            self.save();
            Ok(())
        } else {
            Err(format!("Task with title '{}' not found", title))
        }
    }

    pub fn update_task(&mut self, title: &str, new_task: Task) -> Result<(), String> {
        if let Some(task) = self.tasks.get_mut(title) {
            *task = new_task;
            self.save();
            Ok(())
        } else {
            Err(format!("Task with title '{}' not found", title))
        }
    }

    pub fn delete_task(&mut self, title: &str) -> Result<(), String> {
        if self.tasks.remove(title).is_some() {
            self.save();
            Ok(())
        } else {
            Err(format!("Task with title '{}' not found", title))
        }
    }

    pub fn get_all_tasks(&self) -> Vec<&Task> {
        self.tasks.values().collect()
    }

    pub fn filter_tasks(&self, predicate: &str) -> Result<Vec<&Task>, String> {
        let predicates = parse_predicates(predicate)?;
        Ok(self
            .tasks
            .values()
            .filter(|task| predicates.iter().all(|p| p.matches(task)))
            .collect())
    }

    fn save(&self) {
        let content = serde_json::to_string(&self.tasks).expect("Failed to serialize tasks");
        let tmp_path = self.file_path.with_extension("tmp");
        fs::write(&tmp_path, content).expect("Failed to write to temp file");
        fs::rename(&tmp_path, &self.file_path).expect("Failed to rename temp file");
    }
}

#[derive(Debug, PartialEq)]
enum Predicate {
    Category(String),
    Status(TaskStatus),
    DateBefore(DateTime<Local>),
    DateAfter(DateTime<Local>),
    DescriptionContains(String),
}

impl Predicate {
    fn matches(&self, task: &Task) -> bool {
        match self {
            Predicate::Category(category) => &task.category.0 == category,
            Predicate::Status(status) => &task.status == status,
            Predicate::DateBefore(date) => task.creation_date < *date,
            Predicate::DateAfter(date) => task.creation_date > *date,
            Predicate::DescriptionContains(text) => task.description.contains(text),
        }
    }
}

impl FromStr for Predicate {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.splitn(3, ' ').collect();
        if parts.len() < 3 {
            return Err("Invalid predicate format".to_string());
        }

        match parts[0] {
            "category" => Ok(Predicate::Category(parts[2].to_string())),
            "status" => Ok(Predicate::Status(parts[2].parse()?)),
            "date" => {
                let date = NaiveDateTime::parse_from_str(parts[2], "%Y-%m-%d %H:%M")
                    .map_err(|e| e.to_string())?;
                let date = Local.from_local_datetime(&date).unwrap();
                match parts[1] {
                    "<" => Ok(Predicate::DateBefore(date)),
                    ">" => Ok(Predicate::DateAfter(date)),
                    _ => Err("Invalid date comparison operator".to_string()),
                }
            }
            "description" => {
                if parts[1] != "like" {
                    return Err("Invalid description predicate".to_string());
                }
                Ok(Predicate::DescriptionContains(
                    parts[2].trim_matches('"').to_string(),
                ))
            }
            _ => Err(format!("Unknown predicate type: {}", parts[0])),
        }
    }
}

#[derive(Parser)]
#[command(name = "todo")]
#[command(about = "A simple TODO list CLI application", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new task
    Add {
        title: String,
        description: String,
        #[arg(value_parser = parse_date)]
        date: DateTime<Local>,
        category: String,
    },
    /// Mark a task as done
    Done { title: String },
    /// Update an existing task
    Update { title: String },
    /// Delete a task
    Delete { title: String },
    /// Select tasks based on a predicate
    Select { predicate: String },
    /// List all tasks
    List,
}

fn parse_date(date_str: &str) -> Result<DateTime<Local>, chrono::ParseError> {
    let naive = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M")?;
    Ok(Local.from_local_datetime(&naive).unwrap())
}

fn parse_predicates(predicate: &str) -> Result<Vec<Predicate>, String> {
    let re = Regex::new(r#"(\w+)\s*(=|<|>|like)\s*"([^"]*)""#).unwrap();
    let captures: Vec<_> = re.captures_iter(predicate).collect();

    if captures.is_empty() {
        return Err("Invalid predicate format".to_string());
    }

    captures
        .into_iter()
        .map(|cap| {
            let field = cap[1].to_lowercase();
            let operator = &cap[2];
            let value = cap[3].to_string();

            match (field.as_str(), operator) {
                ("category", "=") => Ok(Predicate::Category(value)),
                ("status", "=") => TaskStatus::from_str(&value)
                    .map(Predicate::Status)
                    .map_err(|e| e.to_string()),
                ("date", "<") => parse_date(&value)
                    .map(Predicate::DateBefore)
                    .map_err(|e| e.to_string()),
                ("date", ">") => parse_date(&value)
                    .map(Predicate::DateAfter)
                    .map_err(|e| e.to_string()),
                ("description", "like") => Ok(Predicate::DescriptionContains(value)),
                _ => Err(format!("Unknown predicate: {}", field)),
            }
        })
        .collect()
}

fn main() {
    let cli = Cli::parse();
    let mut todo_list = TodoList::new(PathBuf::from("tasks.json"));

    match cli.command {
        Commands::Add {
            title,
            description,
            date,
            category,
        } => {
            let task = Task {
                title: title.clone(),
                description,
                creation_date: date,
                category: Category(category),
                status: TaskStatus::Active,
            };
            match todo_list.add_task(task) {
                Ok(_) => println!("Task '{}' added successfully", title),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Done { title } => match todo_list.mark_as_done(&title) {
            Ok(_) => println!("Task '{}' marked as done", title),
            Err(e) => eprintln!("Error: {}", e),
        },
        Commands::Update { title } => {
            if let Some(old_task) = todo_list.tasks.get(&title) {
                println!("Updating task: {}", title);

                println!("Enter new description (press Enter to keep current):");
                let mut new_description = String::new();
                std::io::stdin().read_line(&mut new_description).unwrap();
                let new_description = new_description.trim();
                let new_description = if new_description.is_empty() {
                    old_task.description.clone()
                } else {
                    new_description.to_string()
                };

                println!("Enter new date (YYYY-MM-DD HH:MM) (press Enter to keep current):");
                let mut new_date = String::new();
                std::io::stdin().read_line(&mut new_date).unwrap();
                let new_date = new_date.trim();
                let new_date = if new_date.is_empty() {
                    old_task.creation_date
                } else {
                    parse_date(new_date).unwrap_or(old_task.creation_date)
                };

                println!("Enter new category (press Enter to keep current):");
                let mut new_category = String::new();
                std::io::stdin().read_line(&mut new_category).unwrap();
                let new_category = new_category.trim();
                let new_category = if new_category.is_empty() {
                    old_task.category.clone()
                } else {
                    Category(new_category.to_string())
                };

                println!("Enter new status (on/done) (press Enter to keep current):");
                let mut new_status = String::new();
                std::io::stdin().read_line(&mut new_status).unwrap();
                let new_status = new_status.trim();
                let new_status = if new_status.is_empty() {
                    old_task.status.clone()
                } else {
                    new_status.parse().unwrap_or(old_task.status.clone())
                };

                let new_task = Task {
                    title: title.clone(),
                    description: new_description,
                    creation_date: new_date,
                    category: new_category,
                    status: new_status,
                };

                match todo_list.update_task(&title, new_task) {
                    Ok(_) => println!("Task '{}' updated successfully", title),
                    Err(e) => eprintln!("Error: {}", e),
                }
            } else {
                eprintln!("Error: Task with title '{}' not found", title);
            }
        }
        Commands::Delete { title } => match todo_list.delete_task(&title) {
            Ok(_) => println!("Task '{}' deleted successfully", title),
            Err(e) => eprintln!("Error: {}", e),
        },
        Commands::Select { predicate } => match todo_list.filter_tasks(&predicate) {
            Ok(filtered_tasks) => {
                if filtered_tasks.is_empty() {
                    println!("No tasks match the given predicate.");
                } else {
                    for task in filtered_tasks {
                        println!(
                            "{}: {} ({}) - {} - {}",
                            task.title,
                            task.description,
                            task.status,
                            task.category,
                            task.creation_date
                        );
                    }
                }
            }
            Err(e) => eprintln!("Error filtering tasks: {}", e),
        },
        Commands::List => {
            let all_tasks = todo_list.get_all_tasks();
            if all_tasks.is_empty() {
                println!("No tasks found.");
            } else {
                for task in all_tasks {
                    println!(
                        "{}: {} ({}) - {} - {}",
                        task.title,
                        task.description,
                        task.status,
                        task.category,
                        task.creation_date
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn get_unique_file_path() -> PathBuf {
        let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
        PathBuf::from(format!("test_tasks_{}.json", counter))
    }

    fn cleanup_file(path: &PathBuf) {
        if path.exists() {
            fs::remove_file(path).expect("Failed to remove test file");
        }
    }

    fn setup() -> (TodoList, PathBuf) {
        let file_path = get_unique_file_path();
        let todo_list = TodoList::new(file_path.clone());
        (todo_list, file_path)
    }

    #[test]
    fn test_add_task() {
        let (mut todo_list, file_path) = setup();
        let task = Task::new(
            "Test Task".to_string(),
            "Description".to_string(),
            Category("TestCategory".to_string()),
        );
        assert!(todo_list.add_task(task).is_ok());
        cleanup_file(&file_path);
    }

    #[test]
    fn test_mark_as_done() {
        let (mut todo_list, file_path) = setup();
        let task = Task::new(
            "Test Task".to_string(),
            "Description".to_string(),
            Category("TestCategory".to_string()),
        );
        todo_list.add_task(task).unwrap();
        assert!(todo_list.mark_as_done("Test Task").is_ok());
        assert_eq!(
            todo_list.tasks.get("Test Task").unwrap().status,
            TaskStatus::Done
        );
        cleanup_file(&file_path);
    }

    #[test]
    fn test_delete_task() {
        let (mut todo_list, file_path) = setup();
        let task = Task::new(
            "Test Task".to_string(),
            "Description".to_string(),
            Category("TestCategory".to_string()),
        );
        todo_list.add_task(task).unwrap();
        assert!(todo_list.delete_task("Test Task").is_ok());
        assert!(todo_list.tasks.is_empty());
        cleanup_file(&file_path);
    }

    #[test]
    fn test_filter_tasks() {
        let (mut todo_list, file_path) = setup();
        let task1 = Task::new(
            "Task 1".to_string(),
            "Description 1".to_string(),
            Category("Category1".to_string()),
        );
        let task2 = Task::new(
            "Task 2".to_string(),
            "Description 2".to_string(),
            Category("Category2".to_string()),
        );
        todo_list.add_task(task1).unwrap();
        todo_list.add_task(task2).unwrap();

        let filtered = todo_list.filter_tasks(r#"category = "Category1""#).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].title, "Task 1");

        let filtered = todo_list
            .filter_tasks(r#"description like "Description""#)
            .unwrap();
        assert_eq!(filtered.len(), 2);

        assert!(todo_list.filter_tasks("invalid predicate").is_err());

        cleanup_file(&file_path);
    }

    #[test]
    fn test_predicate_parsing() {
        let (_todo_list, file_path) = setup();
        assert_eq!(
            "category = TestCategory".parse::<Predicate>().unwrap(),
            Predicate::Category("TestCategory".to_string())
        );
        assert_eq!(
            "status = on".parse::<Predicate>().unwrap(),
            Predicate::Status(TaskStatus::Active)
        );
        assert!("date < 2023-05-20 10:00".parse::<Predicate>().is_ok());
        assert_eq!(
            "description like \"test\"".parse::<Predicate>().unwrap(),
            Predicate::DescriptionContains("test".to_string())
        );
        cleanup_file(&file_path);
    }

    #[test]
    fn test_update_task() {
        let (mut todo_list, file_path) = setup();
        let task = Task::new(
            "Test Task".to_string(),
            "Description".to_string(),
            Category("TestCategory".to_string()),
        );
        todo_list.add_task(task).unwrap();

        let updated_task = Task {
            title: "Test Task".to_string(),
            description: "Updated Description".to_string(),
            creation_date: Local::now(),
            category: Category("UpdatedCategory".to_string()),
            status: TaskStatus::Done,
        };

        assert!(todo_list.update_task("Test Task", updated_task).is_ok());

        let updated = todo_list.tasks.get("Test Task").unwrap();
        assert_eq!(updated.description, "Updated Description");
        assert_eq!(updated.category.0, "UpdatedCategory");
        assert_eq!(updated.status, TaskStatus::Done);
        cleanup_file(&file_path);
    }

    #[test]
    fn test_category_fromstr() {
        let (_todo_list, file_path) = setup();
        let category: Category = "TestCategory".parse().unwrap();
        assert_eq!(category.0, "TestCategory");
        cleanup_file(&file_path);
    }

    #[test]
    fn test_taskstatus_fromstr() {
        let (_todo_list, file_path) = setup();
        assert_eq!("on".parse::<TaskStatus>().unwrap(), TaskStatus::Active);
        assert_eq!("done".parse::<TaskStatus>().unwrap(), TaskStatus::Done);
        assert!("invalid".parse::<TaskStatus>().is_err());
        cleanup_file(&file_path);
    }
}
