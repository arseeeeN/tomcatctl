#!/usr/bin/env nu

# Builds the release binary for $env.TARGET and packages it into an archive.
# The archive path is written to $GITHUB_OUTPUT as `archive` for the publish step.
# Adapted from the Nushell release script: https://github.com/nushell/nushell

let bin = 'tomcatctl'
let os = $env.OS
let target = $env.TARGET
# Repo source dir like `/home/runner/work/tomcatctl/tomcatctl`
let src = $env.GITHUB_WORKSPACE
let dist = $'($env.GITHUB_WORKSPACE)/output'
let version = (open Cargo.toml | get package.version)

print $'Debugging info:'
print { version: $version, bin: $bin, os: $os, target: $target, src: $src, dist: $dist }; hr-line -b

let USE_UBUNTU = $os starts-with ubuntu
let USE_WINDOWS = $os starts-with windows

print $'(char nl)Packaging ($bin) v($version) for ($target) in ($src)...'; hr-line -b
if not ('Cargo.lock' | path exists) { cargo generate-lockfile }

print $'Start building ($bin)...'; hr-line

# ----------------------------------------------------------------------------
# Build
# ----------------------------------------------------------------------------
match $target {
    'aarch64-unknown-linux-gnu' => {
        sudo apt-get update
        sudo apt-get install gcc-aarch64-linux-gnu -y
        $env.CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER = 'aarch64-linux-gnu-gcc'
        cargo-build
    }
    _ => {
        # musl-tools to fix 'Failed to find tool. Is `musl-gcc` installed?'
        # Used for x86_64-unknown-linux-musl and for aarch64-unknown-linux-musl on the native ARM runner
        if $USE_UBUNTU and ($target ends-with 'musl') {
            sudo apt-get update
            sudo apt-get install musl-tools -y
        }
        cargo-build
    }
}

# ----------------------------------------------------------------------------
# Prepare the release archive
# ----------------------------------------------------------------------------
let suffix = if $USE_WINDOWS { '.exe' } else { '' }
let executable = $'target/($target)/release/($bin)($suffix)'
print $'Current executable file: ($executable)'

cd $src; mkdir $dist
print $'(char nl)Copying release files...'; hr-line
[LICENSE README.md $executable] | each {|it| cp -v $it $dist } | ignore

# ----------------------------------------------------------------------------
# Create the release archive and send it to output for the following steps
# ----------------------------------------------------------------------------
cd $dist; print $'(char nl)Creating release archive...'; hr-line
let dest = $'($bin)-($version)-($target)'

let archive = if $USE_WINDOWS {
    let archive = $'($dist)/($dest).zip'
    7z a $archive ...(glob *)
    # Workaround for https://github.com/softprops/action-gh-release/issues/280
    $archive | str replace --all '\' '/'
} else {
    let files = (ls | get name)
    mkdir $dest
    $files | each {|it| cp -v $it $dest } | ignore
    print $'(char nl)(ansi g)Archive contents:(ansi reset)'; hr-line; ls $dest | print
    let archive = $'($dist)/($dest).tar.gz'
    tar -czf $archive $dest
    $archive
}

print $'archive: ---> ($archive)'; ls $archive | print
# REF: https://github.blog/changelog/2022-10-11-github-actions-deprecating-save-state-and-set-output-commands/
echo $"archive=($archive)" | save --append $env.GITHUB_OUTPUT

def 'cargo-build' [] {
    cargo build --release --locked --target $target
}

# Print a horizontal line marker
def 'hr-line' [
    --blank-line(-b)
] {
    print $'(ansi g)---------------------------------------------------------------------------->(ansi reset)'
    if $blank_line { char nl }
}
