# TODO List

### Usage:

**Description:**
`cargo run -- help`
`cargo run -- help <command>`

**To list all tasks:**
`cargo run -- list`

**To add a new task:**
`cargo run -- add "Task Title" "Task Description" "2023-05-20 10:00" "cat1"`

**To mark a task as done:**
`cargo run -- done "Task Title"`

**To update a task:**
`cargo run -- update "Task Title"`

**To delete a task:**
`cargo run -- delete "Task Title"`

**To select tasks based on a predicate:**
`cargo run -- select 'date < "2024-12-12 00:00" and category="cat2" and status="on" and description like "Task"'`


----

### Running tests

`cargo test`


----


### TODO:

- To implement `Drop` trait for tests
- To persist tasks to DB

Do what you must...I will watch you.


<p align="center">
<img width="300" height="500" src="https://static.wikia.nocookie.net/elderscrolls/images/b/ba/Imperial_Prison_Guard.png/revision/latest?cb=20131214131751">
</p>