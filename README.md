# Delete ECS Clusters RS

This basic project is used to delete all clusters for ECS(Elastic Container Service) and task definitions in AWS.

## Run

To run in debug mode:
```bash
cargo run
```

To run in release mode:
```bash
cargo run --release
```
To run with logs:
```bash
RUST_LOG=info cargo run --release
```

To see the help add (`-- --help`) ex:
```bash
cargo run --release -- --help
```
PS: There is also options for running multiple regions, regions are in the `regions.txt` file, so regions can be added or removed from there.