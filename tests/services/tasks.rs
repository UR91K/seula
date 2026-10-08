//! `TasksService` tests. Ported from `tests/grpc/tasks.rs` (ADR-0046).

use super::{create_test_project, test_env};

#[tokio::test]
async fn search_tasks_filters_by_text_and_completion() {
    let env = test_env();
    let tasks = &env.services.tasks;
    let project_id = create_test_project(&env, "Test Project", "/path/to/project.als").await;

    tasks
        .create_task(&project_id, "Finish the intro")
        .await
        .unwrap();
    let mix = tasks
        .create_task(&project_id, "Mix the vocals")
        .await
        .unwrap();
    tasks
        .create_task(&project_id, "Master the track")
        .await
        .unwrap();
    tasks.update_task(&mix.0, None, Some(true)).await.unwrap();

    let (found, total) = tasks
        .search_tasks(&project_id, "intro", None, None, None, None)
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(total, 1);
    assert_eq!(found[0].1, "Finish the intro");

    let (completed, total) = tasks
        .search_tasks(&project_id, "", None, None, Some(true), None)
        .await
        .unwrap();
    assert_eq!(completed.len(), 1);
    assert_eq!(total, 1);
    assert_eq!(completed[0].1, "Mix the vocals");
    assert!(completed[0].2, "the row carries its completed flag");

    let (pending, total) = tasks
        .search_tasks(&project_id, "", None, None, None, Some(true))
        .await
        .unwrap();
    assert_eq!(pending.len(), 2);
    assert_eq!(total, 2);
    assert!(pending.iter().all(|task| !task.2));
}

#[tokio::test]
async fn task_statistics_count_one_project_and_all() {
    let env = test_env();
    let tasks = &env.services.tasks;
    let project_id = create_test_project(&env, "Test Project", "/path/to/project.als").await;

    tasks.create_task(&project_id, "Task 1").await.unwrap();
    let task2 = tasks.create_task(&project_id, "Task 2").await.unwrap();
    let task3 = tasks.create_task(&project_id, "Task 3").await.unwrap();
    tasks.update_task(&task2.0, None, Some(true)).await.unwrap();
    tasks.update_task(&task3.0, None, Some(true)).await.unwrap();

    let project = tasks.get_task_statistics(Some(&project_id)).await.unwrap();
    assert_eq!(project.total_tasks, 3);
    assert_eq!(project.completed_tasks, 2);
    assert_eq!(project.pending_tasks, 1);
    assert!((project.completion_rate - 66.66666666666667).abs() < 0.1);

    let global = tasks.get_task_statistics(None).await.unwrap();
    assert_eq!(global.total_tasks, 3);
    assert_eq!(global.completed_tasks, 2);
    assert_eq!(global.pending_tasks, 1);
    assert!((global.completion_rate - 66.66666666666667).abs() < 0.1);
}

#[tokio::test]
async fn search_tasks_paginates() {
    let env = test_env();
    let tasks = &env.services.tasks;
    let project_id = create_test_project(&env, "Test Project", "/path/to/project.als").await;
    for i in 1..=5 {
        tasks
            .create_task(&project_id, &format!("Task {}", i))
            .await
            .unwrap();
    }

    let (first, total) = tasks
        .search_tasks(&project_id, "Task", Some(3), Some(0), None, None)
        .await
        .unwrap();
    assert_eq!(first.len(), 3);
    assert_eq!(total, 5);

    let (rest, total) = tasks
        .search_tasks(&project_id, "Task", Some(3), Some(3), None, None)
        .await
        .unwrap();
    assert_eq!(rest.len(), 2, "the remaining two");
    assert_eq!(total, 5);
}
