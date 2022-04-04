# Building `ros2_rust` packages

In this guide, the Foxy distribution of ROS 2 is used, but newer distributions can be used by simply replacing 'foxy' with the distribution name everywhere.

## Environment setup

Building `rclrs` requires a standard [ROS 2 installation](https://docs.ros.org/en/foxy/Installation.html), and a few Rust-specific extensions.
These extensions are: `colcon-cargo`, `colcon-ros-cargo`, `cargo-ament-build`. The former two are `colcon` plugins, and the latter is a `cargo` plugin.

It is recommended to use the pre-made Docker image that contains all the necessary dependencies.
If you do not want to use Docker, see the `Dockerfile` in the `ros2_rust` repo for how dependencies can be installed.

### Using the Docker image

With the `ros2_rust` repo root as the working directory, build the Docker image with

```shell
docker build -t ros2_rust_container - < Dockerfile
```

and then run it with

```shell
docker run --rm -it --volume $(pwd):/workspace ros2_rust_container /bin/bash
```

### Importing repositories

`ros2_rust` also expects a few other repositories to be in the workspace. They can be imported with the help of a `repos` file contained in `ros2_rust`, like this:

```shell
# Make sure to run this in the workspace directory
vcs import src < src/ros2_rust/ros2_rust_foxy.repos
```

The new repos are now in `src/ros2`.

The repositories should very rarely change, so you typically only need to do this once.

### Sourcing the ROS 2 installation

Before building, the ROS 2 installation, and ideally _only_ the ROS 2 installation, should be sourced.
In the pre-made Docker image, the ROS 2 installation is already sourced.

If you are not sure, you can check that the `$AMENT_PREFIX_PATH` environment variable contains only the ROS 2 installation.
If it is empty, you need to source the ROS 2 installation:

```shell
. /opt/ros/foxy/setup.sh
````

If `$AMENT_PREFIX_PATH` contains undesired paths, exit and reopen your shell.

## Building with `colcon`

To verify that you've correctly installed dependencies and sourced your ROS 2 installation, you should be able to run

```shell
colcon list
```
and see the package `rclrs` listed with build type `ament_cargo`.

The basic steps are as simple as

```shell
colcon build --packages-up-to $YOUR_PACKAGE
```

where `$YOUR_PACKAGE` could be e.g. `rclrs_examples`.

It's normal to see a `Some selected packages are already built in one or more underlay workspace` warning. This is because the standard message definitions that are part of ROS 2 need to be regenerated in order to create Rust bindings.

Don't start two build processes involving Rust packages in parallel; they might overwrite each other's `.cargo/config.toml`.

Do not build in a shell where you've sourced `setup.sh`.
When using a Docker container, it's convenient to install `tmux` inside it and open separate windows or panes for building and running.

See `colcon build --help` for a complete list of options.

A clean build will always be much slower than an incremental rebuild.

### Using a workspace
Unfortunately, `colcon` and `cargo` both see themselves as being responsible for building each package and all its dependencies.


## Building with `cargo`
As an alternative to `colcon`, Rust packages can be built with pure `cargo`.

However, this will not work out of the box, since the `Cargo.toml` files contain dependencies like `rclrs = "*"`, even though `rclrs` is not published on crates.io. This is intentional and follows ROS 2's principle for packages to reference their dependencies only with their name, and not with their path. At build-time, these dependencies are resolved to a path to the local package by `colcon`, and written into `.cargo/config.toml`. Therefore, the package in question should be built with `colcon` once, and after that `cargo` will be able to use the `.cargo/config.toml` file to find all dependencies.

### Setup with colcon
Unfortunately, `cargo` may sometimes print messages saying

> warning: Patch `rclrs v0.1.0 (/home/nikolai.morin/ros2_rust/install/rclrs/share/rclrs/rust)` was not used in the crate graph.

This can be ignored.

### Pure `cargo`

A second catch is that `cargo` message packages link against native libraries. A convenient way to ensure that they are found is to also source the setup script produced by `colcon`.

As an example, here is how to build `rclcrs_examples` with `cargo`:

```
# Initial build of the package with colcon
# Compare .cargo/config.toml with and without the --lookup-in-workspace flag to see its effect
colcon build --packages-up-to rclrs_examples --lookup-in-workspace
# Source the install directory
. install/setup.sh
cd rclrs_examples
# Run cargo build, or cargo check, cargo doc, etc.
cargo build
```

## Troubleshooting
If something goes very wrong and you want to start fresh, make sure to delete all `install*`, `build*` and `.cargo` directories. Also, make sure your terminal does not have any install sourced (check with `echo $AMENT_PREFIX_PATH`, which should be empty).

### Package identification
When you forgot to source the ROS 2 installation, you'll get this error:

> ERROR:colcon.colcon_core.package_identification:Exception in package identification extension 'python_setup_py'

Source the ROS 2 installation and try again.

### Failed to resolve patches
When you've deleted your `install` directory but not your `.cargo` directory, you'll get this error:

> error: failed to resolve patches for `https://github.com/rust-lang/crates.io-index`

Delete the `.cargo` directory, and 


cargo ament-build
rust-analyzer
