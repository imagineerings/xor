#!/usr/bin/env python3

import json
import os
import pathlib
import subprocess


DIRECTORY = pathlib.Path(__file__).resolve().parent
CURRENT_IMAGE = "ghcr.io/zed-industries/collaboration@sha256:" + "1" * 64
PREVIOUS_IMAGE = "ghcr.io/zed-industries/collaboration@sha256:" + "2" * 64


def render(image: str, files: tuple[str, ...] = ("compose.yaml",)) -> dict:
    environment = os.environ.copy()
    environment.update(
        {
            "COLLABORATION_IMAGE": image,
            "COLLABORATION_PUBLIC_URL": "https://collaboration.example.test",
            "POSTGRES_PASSWORD": "postgres-canary",
            "REDIS_PASSWORD": "redis-canary",
            "COLLABORATION_OBJECT_ACCESS_KEY": "object-access-canary",
            "COLLABORATION_OBJECT_SECRET_KEY": "object-secret-canary",
            "ZED_CLOUD_INTERNAL_API_KEY": "internal-api-canary",
        }
    )
    command = ["docker", "compose"]
    for file in files:
        command.extend(("-f", file))
    command.extend(("--profile", "validation", "config", "--format", "json"))
    result = subprocess.run(
        command,
        cwd=DIRECTORY,
        env=environment,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def main() -> None:
    document = render(CURRENT_IMAGE)
    services = document["services"]
    require(
        set(services)
        == {
            "collaboration",
            "collaboration-readiness",
            "postgres",
            "redis",
            "object-store",
            "object-store-init",
        },
        "unexpected service inventory",
    )

    collaboration = services["collaboration"]
    require(collaboration["image"] == CURRENT_IMAGE, "current image selection drifted")
    require(
        collaboration["depends_on"]
        == {
            "object-store": {"condition": "service_healthy", "required": True},
            "object-store-init": {
                "condition": "service_completed_successfully",
                "required": True,
            },
            "postgres": {"condition": "service_healthy", "required": True},
            "redis": {"condition": "service_healthy", "required": True},
        },
        "Collab dependency health ordering drifted",
    )
    environment = collaboration["environment"]
    for canonical in (
        "COLLABORATION_PUBLIC_URL",
        "COLLABORATION_DATABASE_URL",
        "COLLABORATION_REDIS_URL",
        "COLLABORATION_OBJECT_ENDPOINT",
        "COLLABORATION_OBJECT_REGION",
        "COLLABORATION_OBJECT_BUCKET",
        "COLLABORATION_OBJECT_ACCESS_KEY",
        "COLLABORATION_OBJECT_SECRET_KEY",
        "COLLABORATION_OBJECT_ADDRESSING_STYLE",
        "COLLABORATION_GIT_REPOSITORY_PATH",
        "COLLABORATION_REPLICA_COUNT",
        "COLLABORATION_PUSH_ENABLED",
        "COLLABORATION_PAIRING_ENABLED",
        "COLLABORATION_RELAY_MESH_ENABLED",
    ):
        require(canonical in environment, f"missing canonical setting {canonical}")
    require(
        environment["DATABASE_URL"] == environment["COLLABORATION_DATABASE_URL"],
        "runtime and canonical database bindings differ",
    )
    require(
        environment["BLOB_STORE_URL"] == environment["COLLABORATION_OBJECT_ENDPOINT"],
        "runtime and canonical object endpoints differ",
    )
    require(
        environment["COLLABORATION_OBJECT_ENDPOINT"] == "http://127.0.0.1:9000",
        "the local object endpoint left its admitted loopback boundary",
    )
    require(
        collaboration["network_mode"] == "service:object-store",
        "Collab no longer shares the object-store loopback namespace",
    )

    for service in ("collaboration", "postgres", "redis", "object-store"):
        require("healthcheck" in services[service], f"{service} has no healthcheck")
    require(
        services["collaboration-readiness"]["profiles"] == ["validation"],
        "readiness probe must remain private to explicit validation",
    )
    require(
        services["collaboration-readiness"]["depends_on"]["collaboration"]["condition"]
        == "service_healthy",
        "readiness probe must follow Collab liveness",
    )

    expected_volumes = {
        "collaboration-postgres",
        "collaboration-redis",
        "collaboration-objects",
        "collaboration-git",
    }
    require(set(document["volumes"]) == expected_volumes, "canonical volume inventory drifted")

    rollback = render(PREVIOUS_IMAGE)
    require(
        rollback["services"]["collaboration"]["image"] == PREVIOUS_IMAGE,
        "prior immutable image did not replace the current image",
    )
    for service in expected_volumes:
        require(
            rollback["volumes"][service] == document["volumes"][service],
            f"rollback changed volume {service}",
        )

    smoke = render(CURRENT_IMAGE, ("compose.yaml", "compose.smoke.yaml"))
    require(
        smoke["services"]["collaboration"]["image"] == "busybox:1.37.0-musl",
        "smoke overlay can invoke a release image",
    )
    require(
        smoke["services"]["object-store"].get("ports") is None,
        "isolated smoke publishes a host port",
    )
    require(
        smoke["services"]["collaboration-readiness"]["command"][-1]
        == "http://object-store:8080/healthz",
        "smoke readiness does not use the shared namespace",
    )

    script = (DIRECTORY / "run.sh").read_text()
    require("@sha256:[0-9a-f]{64}" in script, "rollback digest gate is missing")
    require("--no-deps collaboration" in script, "rollback could recreate dependencies")
    require("verify_readiness" in script, "rollback does not verify readiness")
    print("collaboration Compose contract checks passed")


if __name__ == "__main__":
    main()
