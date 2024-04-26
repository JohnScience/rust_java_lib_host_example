# rust_java_lib_host_example

An example of a Java library powered by compiled native Rust code (for the host architecture).

Based largely on the article ["Embedding Rust code in Java Jar for distribution"](https://www.infinyon.com/blog/2021/05/java-client/) by Sebastian Imlay @ Infinyon.

Before reading this, it is **strongly recommended** that you read the article linked above first. Many things mentioned here won't make sense without the context provided by the article.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install).
- [Java](https://www.oracle.com/java/technologies/javase-jdk11-downloads.html).
- [Gradle](https://gradle.org/install/).
- LLVM due to dependncy on [`bindgen`](https://crates.io/crates/bindgen), which requires `libclang` to be installed. See the [requirements](https://rust-lang.github.io/rust-bindgen/requirements.html).
- Possibly JNI headers? [^1].

## Minor differences, changes, issues, and their effects

- When creating a library, I used the underscore instead of the hyphen in the name. [Both options are correct in Rust at the moment of writing](https://www.reddit.com/r/rust/comments/194clzq/underscores_vs_dashes_in_crate_names/). However, when picking the "source package", Gradle treats underscores and hyphens differently, so the the default "source package" for my library ended up being `my_java_lib` instead of `my.java.lib`. You can manually enter the "source package" during the `gradle init` setup just like I did.
- The order of choices for the build script DSL (Groovy and Kotlin) was swapped in my case.
- I renamed `src/java_glue.rs.in` to `src/java_glue.rs` to get at least the basic syntax highlighting in my editor. It does trigger the `rust-analyzer(unlinked-file)`, yet it is the smaller evil. Unfortunately, disabling the lint for this specific file is impossible at the time of writing. According to [`kpreid`](https://github.com/kpreid), there is a `rust-analyzer.diagnostics.disabled` setting for Rust Analyzer, yet it's currently impossible to provide settings per-project ([issue](https://github.com/rust-lang/rust-analyzer/issues/13529)), let alone per-file.
- I couldn't run the `cargo expand` command as described in the article. I got multiple screens of errors. Updating `cargo expand` via `cargo install cargo-expand` helped.
- The detail so minor that I even considered to omit it. Running `./<...>` (e.g. `./gradlew test`) doesn't work on Windows. The recommendation is to run it as `.\gradlew test` instead.
- Placing `FooTest.java` in `lib/src/test/java/my/java/lib/FooTest.java` didn't work and is probably incorrect. I moved it to `lib/src/test/java/FooTest.java` and it worked.

I believe this is because the `JUnit` is now added as a `testImplementation` dependency:

`lib/build.gradle`:

```gradle
// ...

dependencies {
    // Use JUnit test framework.
    testImplementation libs.junit

    // This dependency is exported to consumers, that is to say found on their compile classpath.
    api libs.commons.math3

    // This dependency is used internally, and not exposed to consumers on their own compile classpath.
    implementation libs.guava
}

// ...
```

- I found that `cargo expand` was *very* long, so I used `cargo expand > output.txt` to see the output. Among with other files, I also added `output.txt` to the `.gitignore` file to avoid commiting the file accidentally. In my case, the file was 7,995 lines long.
- I had to slightly alter the `lib/build.gradle`.

Between the

```gradle
// ...
tasks.withType(JavaCompile) {
    compileTask -> compileTask.dependsOn "rust-deploy"
}
// ...
```

and the

```gradle
// ...
sourceSets {
    main {
        java {
            srcDir 'src/main/java'
        }
        resources {
            srcDir 'rust-lib'
        }
    }
}
// EOF

```

I inserted the following:

```gradle
// Note: the original code was missing the following 4 lines of code
tasks.withType(ProcessResources) {
    dependsOn "rust-deploy"
    mustRunAfter "rust-deploy"
}
```

Without them, gradle errored.

Also, since I use Windows (with WSL for certain tasks), I had to copy over the `.dll`s.

Before:

```gradle
tasks.create(name: "rust-deploy", type: Sync, dependsOn: "cargo-build") {
    from "${project.ext.cargo_target_directory}/release"
    include "*.dylib","*.so"
    into "rust-lib/"
}
```

After:

```gradle
tasks.create(name: "rust-deploy", type: Sync, dependsOn: "cargo-build") {
    from "${project.ext.cargo_target_directory}/release"
    include "*.dylib","*.so","*.dll"
    into "rust-lib/"
}
```

- I had to update the "foreign code" in `src/java_glue.rs` as such:

```rust
foreign_class!(class Foo {
    self_type Foo;
    constructor Foo::new(_: i32) -> Foo;
    fn Foo::set_field(&mut self, _: i32);
    fn Foo::val(&self) -> i32;
    foreign_code r#"
        static {
            try {
                String osName = System.getProperty("os.name").toLowerCase();
                String libExtension = "";
                String prefix = "";

                if (osName.contains("mac")) {
                    prefix = "lib";
                    libExtension = ".dylib";
                } else if (osName.contains("win")) {
                    // prefix remains empty
                    libExtension = ".dll";
                } else {
                    prefix = "lib";
                    libExtension = ".so";
                }

                NativeUtils.loadLibraryFromJar("/" + prefix + "my_java_lib" + libExtension);
            } catch (java.io.IOException e) {
                e.printStackTrace();
            }
        }
    "#;
});
```

Note that on Windows, cargo doesn't prefix the `.dll` files with `lib`, so the `prefix` variable remains empty. More one thie library loading later in the README.

- I added the following to the `.gitignore`:

```gitignore
# Ignore generated java files
lib/src/main/java/my/java/lib/*
# NativeUtils.java is not generated, so it should be included
!lib/src/main/java/my/java/lib/NativeUtils.java
# Ignore the directory with the generated dynamically linked library (.dll, .so, .dylib)
lib/rust-lib
# Ignore output.txt file generated with `cargo expand > output.txt`
output.txt
```

## Ruminations

Overall, this is a reasonably simple process. However, there are a few things that could be improved:

- The selection of the dll/dylib/so file should be "librarified". Maybe the `cargo-artifact-name` crate with a function `artifact_name(crate_name, artifact_kind, target)` could be created. This would allow for a more robust and less error-prone way of selecting the correct artifact. Also, it can allow for a targeted cooperation on the task of bridging Rust and Java. However, the problem is that "foreign_code" seems to deal with a literal (or a stringly-typed expression) **in Java**. For that purpuse, another function which would generate the correct code *in Java* would need to be created and ideally it also has to account for the situation where the target can be selected from a predefined set of targets (possibly, the set can be comprised of the single target, which can eliminate the need for any branching).

- The article deals only with embedding a single dynamically-linked library (.dll/.so/.dylib) for the host platform. However, the case of a `.jar` with native code for multiple architectures (e.g. for different architectures on Android) should probably be considered as well. This involves the problems of cross-compilation and target-selection.

- The case for Android should be covered specfically. Luckily, `flapigen` has an instruction with a dedicated `Java/Android` paragraph [here](https://dushistov.github.io/flapigen-rs/java-android-example.html).

- The `NativeUtils` class, according to article

> ...looks at the jar, unzips it in a temp directory, and then calls System.load.

However, unziping it in a temporary directory maybe unncecessary due to possibility of reflective loading. I already implemented a simple reflective PECOFF DLL loader (<https://github.com/JohnScience/reflective_pe_dll_loader/>). This may be more performant to unzip the library in memory and load it from there. This would allow to circumvent the slow disk I/O and the need to clean up the temporary directory. However, platforms like Android may not allow for reflective loading of native libraries.

- It should be possible to compile Rust to WASM and then translate it to Java bytecode with <https://github.com/cretz/asmble>. This would allow for a more portable solution at the cost of performance and limitations on which Rust libraries can be used. It can also be interesting how Rust's (1) WASM and (2) WASM->Java bytecode libraries can be used with "host functions" that would allow for access to the Java environment, thereby allowing to "escape the sandbox".

- It would be interesting the cooperation between [`flapigen`](https://github.com/Dushistov/flapigen-rs), [`diplomat`](https://github.com/rust-diplomat/diplomat), and [`specta`](https://github.com/oscartbeaumont/specta).

- It would be awesome to see the creation of cross-language interoperability WG.

- It seems that there's quite a lot of boilerplate in `build.gradle`. Maybe a Gradle plugin could be created to make the code more concise and less error-prone.

## Also see

- SWIG, a software development tool that connects programs written in C and C++ with a variety of high-level programming languages: <https://www.swig.org/>.
- ["Wrapping a C library in Java"](https://nachtimwald.com/2017/06/06/wrapping-a-c-library-in-java/) article by John Schember.

[^1]: I didn't have to install these on my machine, but it's possible that you might need to.
