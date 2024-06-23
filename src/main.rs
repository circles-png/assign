use google_classroom1::{
    api::{ModifyCourseWorkAssigneesRequest, ModifyIndividualStudentsOptions, Scope},
    hyper::Client,
    hyper_rustls::HttpsConnectorBuilder,
    oauth2::{ConsoleApplicationSecret, InstalledFlowAuthenticator, InstalledFlowReturnMethod},
    Classroom,
};
use google_sheets4::Sheets;
use inquire::{MultiSelect, Text};
use itertools::Itertools;
use serde_json::from_str;

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() {
    let secret: ConsoleApplicationSecret = from_str(include_str!("secret.json")).unwrap();
    let auth = InstalledFlowAuthenticator::builder(
        secret.web.unwrap(),
        InstalledFlowReturnMethod::HTTPPortRedirect(8080),
    )
    .persist_tokens_to_disk("tokens")
    .build()
    .await
    .unwrap();
    let client = Client::builder().build(
        HttpsConnectorBuilder::new()
            .with_native_roots()
            .https_or_http()
            .enable_http1()
            .build(),
    );
    let classroom = Classroom::new(client.clone(), auth.clone());
    let sheets = Sheets::new(client, auth);

    let courses = classroom
        .courses()
        .list()
        .doit()
        .await
        .unwrap()
        .1
        .courses
        .unwrap();
    let course = courses
        .iter()
        .find(|course| course.name == Some(include_str!("name").trim().to_string()))
        .unwrap();

    let id = include_str!("sheet").trim();

    let people = sheets
        .spreadsheets()
        .values_batch_get(id)
        .add_ranges("B2:B")
        .add_ranges("E2:E")
        .doit()
        .await
        .unwrap()
        .1
        .value_ranges
        .unwrap()
        .into_iter()
        .map(|range| {
            range
                .values
                .unwrap()
                .iter()
                .map(|row| row.first().unwrap().as_str().unwrap().to_string())
                .collect_vec()
        })
        .collect_vec();
    let assignments = classroom
        .courses()
        .course_work_list(course.id.as_ref().unwrap())
        .add_scope(Scope::CourseworkStudent)
        .add_course_work_states("PUBLISHED")
        .add_course_work_states("DRAFT")
        .doit()
        .await
        .unwrap()
        .1
        .course_work
        .unwrap();
    let assignments = MultiSelect::new(
        "Assignments",
        assignments
            .iter()
            .map(|assignment| assignment.title.as_ref().unwrap().to_string())
            .collect_vec(),
    )
    .raw_prompt()
    .unwrap()
    .iter()
    .map(|option| assignments[option.index].clone().id)
    .collect_vec();
    let role = Text::new("Role to filter by").prompt().unwrap();
    let people = people
        .first()
        .unwrap()
        .iter()
        .zip(people.last().unwrap().iter())
        .filter(|person| person.1.contains(&role) && !include_str!("teachers").contains(person.0))
        .collect_vec();

    for id in assignments {
        classroom
            .courses()
            .course_work_modify_assignees(
                ModifyCourseWorkAssigneesRequest {
                    assignee_mode: Some("INDIVIDUAL_STUDENTS".to_string()),
                    modify_individual_students_options: Some(ModifyIndividualStudentsOptions {
                        add_student_ids: Some(
                            people.iter().map(|person| person.0.clone()).collect_vec(),
                        ),
                        remove_student_ids: None,
                    }),
                },
                course.id.as_ref().unwrap(),
                id.as_ref().unwrap(),
            )
            .doit()
            .await
            .unwrap();
    }
}
