#![type_length_limit = "2097152"]
extern crate bollard;
extern crate failure;
extern crate futures;
extern crate hyper;
#[cfg(unix)]
extern crate hyperlocal;
extern crate tokio;

use futures::Stream;
use hyper::client::connect::Connect;
use hyper::rt::Future;
use tokio::runtime::Runtime;

use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions,
    WaitContainerOptions,
};
use bollard::image::*;
use bollard::Docker;

use std::default::Default;

#[macro_use]
pub mod common;
use common::*;

fn create_image_test<C>(docker: Docker<C>)
where
    C: Connect + Sync + 'static,
{
    let rt = Runtime::new().unwrap();
    let future = chain_create_image_hello_world(docker.chain());
    run_runtime(rt, future);
}

fn search_images_test<C>(docker: Docker<C>)
where
    C: Connect + Sync + 'static,
{
    let rt = Runtime::new().unwrap();
    let future = docker
        .chain()
        .search_images(SearchImagesOptions {
            term: "hello-world",
            ..Default::default()
        })
        .map(|(docker, result)| {
            assert!(result
                .into_iter()
                .any(|api_image| &api_image.name == "hello-world"));
            docker
        });

    run_runtime(rt, future);
}

fn inspect_image_test<C>(docker: Docker<C>)
where
    C: Connect + Sync + 'static,
{
    let image = move || {
        if cfg!(windows) {
            format!("{}hello-world:nanoserver", registry_http_addr())
        } else {
            format!("{}hello-world:linux", registry_http_addr())
        }
    };

    let rt = Runtime::new().unwrap();
    let future = chain_create_image_hello_world(docker.chain())
        .and_then(move |docker| docker.inspect_image(&image()))
        .map(move |(docker, result)| {
            assert!(result
                .repo_tags
                .into_iter()
                .any(|repo_tag| repo_tag == image().to_string()));
            docker
        });

    run_runtime(rt, future);
}

fn list_images_test<C>(docker: Docker<C>)
where
    C: Connect + Sync + 'static,
{
    let image = move || {
        if cfg!(windows) {
            format!("{}hello-world:nanoserver", registry_http_addr())
        } else {
            format!("{}hello-world:linux", registry_http_addr())
        }
    };

    let rt = Runtime::new().unwrap();
    let future = chain_create_image_hello_world(docker.chain())
        .and_then(move |docker| {
            docker.list_images(Some(ListImagesOptions::<String> {
                all: true,
                ..Default::default()
            }))
        })
        .map(move |(docker, result)| {
            assert!(result.into_iter().any(|api_image| api_image
                .repo_tags
                .unwrap_or(vec![String::new()])
                .into_iter()
                .any(|repo_tag| repo_tag == image().to_string())));
            docker
        });

    run_runtime(rt, future);
}

fn image_history_test<C>(docker: Docker<C>)
where
    C: Connect + Sync + 'static,
{
    let image = move || {
        if cfg!(windows) {
            format!("{}hello-world:nanoserver", registry_http_addr())
        } else {
            format!("{}hello-world:linux", registry_http_addr())
        }
    };

    let rt = Runtime::new().unwrap();
    let future = chain_create_image_hello_world(docker.chain())
        .and_then(move |docker| docker.image_history(&image()))
        .map(move |(docker, result)| {
            assert!(result.into_iter().take(1).any(|history| history
                .tags
                .unwrap_or(vec![String::new()])
                .into_iter()
                .any(|tag| tag == image().to_string())));
            docker
        });

    run_runtime(rt, future);
}

fn prune_images_test<C>(docker: Docker<C>)
where
    C: Connect + Sync + 'static,
{
    rt_exec!(
        docker.prune_images(None::<PruneImagesOptions<String>>),
        |_| ()
    );
}

fn remove_image_test<C>(docker: Docker<C>)
where
    C: Connect + Sync + 'static,
{
    let image = move || {
        if cfg!(windows) {
            format!("{}hello-world:nanoserver", registry_http_addr())
        } else {
            format!("{}hello-world:linux", registry_http_addr())
        }
    };

    let rt = Runtime::new().unwrap();
    let future = chain_create_image_hello_world(docker.chain())
        .and_then(move |docker| {
            docker.remove_image(
                &image(),
                Some(RemoveImageOptions {
                    noprune: true,
                    ..Default::default()
                }),
            )
        })
        .map(move |(docker, result)| {
            assert!(result.into_iter().any(|s| match s {
                RemoveImageResults::RemoveImageUntagged { untagged } => untagged == image(),
                _ => false,
            }));
            docker
        });

    run_runtime(rt, future);
}

fn commit_container_test<C>(docker: Docker<C>)
where
    C: Connect + Sync + 'static,
{
    let image = move || {
        if cfg!(windows) {
            format!("{}microsoft/nanoserver", registry_http_addr())
        } else {
            format!("{}alpine", registry_http_addr())
        }
    };

    let cmd = if cfg!(windows) {
        Some(vec![
            "cmd.exe".to_string(),
            "/C".to_string(),
            "copy".to_string(),
            "nul".to_string(),
            "bollard.txt".to_string(),
        ])
    } else {
        Some(vec!["touch".to_string(), "/bollard.txt".to_string()])
    };

    let rt = Runtime::new().unwrap();
    let future = chain_create_image_hello_world(docker.chain())
        .and_then(move |docker| {
            docker.create_container(
                Some(CreateContainerOptions {
                    name: "integration_test_commit_container",
                }),
                Config {
                    cmd: cmd,
                    image: Some(image()),
                    ..Default::default()
                },
            )
        })
        .and_then(move |(docker, _)| {
            docker.start_container(
                "integration_test_commit_container",
                None::<StartContainerOptions<String>>,
            )
        })
        .and_then(move |(docker, _)| {
            docker.wait_container(
                "integration_test_commit_container",
                None::<WaitContainerOptions<String>>,
            )
        })
        .and_then(move |(docker, _)| {
            docker.commit_container(
                CommitContainerOptions {
                    container: "integration_test_commit_container",
                    repo: "integration_test_commit_container_next",
                    pause: true,
                    ..Default::default()
                },
                Config::<String> {
                    ..Default::default()
                },
            )
        })
        .and_then(move |(docker, _)| {
            docker.create_container(
                Some(CreateContainerOptions {
                    name: "integration_test_commit_container_next",
                }),
                Config {
                    image: Some("integration_test_commit_container_next"),
                    cmd: if cfg!(windows) {
                        Some(vec!["cmd.exe", "/C", "dir", "bollard.txt"])
                    } else {
                        Some(vec!["ls", "/bollard.txt"])
                    },
                    ..Default::default()
                },
            )
        })
        .and_then(move |(docker, _)| {
            docker.start_container(
                "integration_test_commit_container_next",
                None::<StartContainerOptions<String>>,
            )
        })
        .and_then(move |(docker, _)| {
            docker.wait_container(
                "integration_test_commit_container_next",
                None::<WaitContainerOptions<String>>,
            )
        })
        .map(move |(docker, stream)| {
            stream
                .take(1)
                .into_future()
                .map(|(head, _)| {
                    let first = head.unwrap();
                    if let Some(error) = first.error {
                        println!("{}", error.message);
                    }
                    assert_eq!(first.status_code, 0);
                    docker
                })
                .or_else(|e| {
                    println!("{}", e.0);
                    Err(e.0)
                })
        })
        .flatten()
        .and_then(move |docker| {
            docker.remove_container(
                "integration_test_commit_container_next",
                None::<RemoveContainerOptions>,
            )
        })
        .and_then(move |(docker, _)| {
            docker.remove_image(
                "integration_test_commit_container_next",
                None::<RemoveImageOptions>,
            )
        })
        .and_then(move |(docker, _)| {
            docker.remove_container(
                "integration_test_commit_container",
                None::<RemoveContainerOptions>,
            )
        });

    run_runtime(rt, future);
}

#[test]
fn integration_test_search_images() {
    connect_to_docker_and_run!(search_images_test);
}

#[test]
fn integration_test_create_image() {
    connect_to_docker_and_run!(create_image_test);
}

#[test]
fn integration_test_inspect_image() {
    connect_to_docker_and_run!(inspect_image_test);
}

#[test]
fn integration_test_image_create() {
    connect_to_docker_and_run!(list_images_test);
}

#[test]
fn integration_test_image_history() {
    connect_to_docker_and_run!(image_history_test);
}

#[test]
fn integration_test_prune_images() {
    connect_to_docker_and_run!(prune_images_test);
}

#[test]
fn integration_test_remove_image() {
    connect_to_docker_and_run!(remove_image_test);
}

#[test]
fn integration_test_commit_container() {
    connect_to_docker_and_run!(commit_container_test);
}
