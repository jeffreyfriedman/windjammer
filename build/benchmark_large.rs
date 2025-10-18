use std::fmt::Write;

#[derive(Debug, Clone)]
struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
}

#[derive(Debug, Clone)]
struct Project {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub owner_id: i64,
}

#[derive(Debug, Clone)]
struct Task {
    pub id: i64,
    pub project_id: i64,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub created_by: i64,
}

fn main() {
    println!("TaskFlow Large-Scale Benchmark - Windjammer");
    let user_count = benchmark_users();
    let project_count = benchmark_projects();
    let task_count = benchmark_tasks();
    let string_count = benchmark_string_formatting();
    println!("User operations: {}", user_count);
    println!("Project operations: {}", project_count);
    println!("Task operations: {}", task_count);
    println!("String operations: {}", string_count);
    println!("Benchmark complete!")
}

#[inline]
fn benchmark_users() -> i64 {
    let mut count = 0;
    let username = String::from("user");
    let email = String::from("user@example.com");
    for i in 0..10000 {
        let user = User { id: i, username: username.clone(), email: email.clone() };
        if user.id >= 0 {
            count += 1;
        }
    }
    count
}

#[inline]
fn benchmark_projects() -> i64 {
    let mut count = 0;
    let name = String::from("Project");
    let description = String::from("Description");
    for i in 0..5000 {
        let project = Project { id: i, name: name.clone(), description: description.clone(), owner_id: i % 10 };
        if project.id >= 0 {
            count += 1;
        }
    }
    count
}

#[inline]
fn benchmark_tasks() -> i64 {
    let mut count = 0;
    let title = String::from("Task");
    let description = String::from("Description");
    let status = String::from("open");
    let priority = String::from("medium");
    for i in 0..20000 {
        let task = Task { id: i, project_id: i % 50, title: title.clone(), description: description.clone(), status: status.clone(), priority: priority.clone(), created_by: i % 100 };
        if task.id >= 0 {
            count += 1;
        }
    }
    count
}

#[inline]
fn benchmark_string_formatting() -> i64 {
    let mut count = 0;
    for i in 0..10000 {
        let user_msg = {
            let mut __s = String::with_capacity(64);
            write!(&mut __s, "User #{}: username={}, email={}", i, "testuser", "test@example.com").unwrap();
            __s
        };
        let project_msg = {
            let mut __s = String::with_capacity(64);
            write!(&mut __s, "Project {}: name={}, owner={}", i, "Project Alpha", i % 1000).unwrap();
            __s
        };
        let task_msg = {
            let mut __s = String::with_capacity(64);
            write!(&mut __s, "Task {}: title={}, status={}, priority={}", i, "Feature", "Active", "High").unwrap();
            __s
        };
        if user_msg.len() > 0 && project_msg.len() > 0 && task_msg.len() > 0 {
            count += 1;
        }
    }
    count
}

