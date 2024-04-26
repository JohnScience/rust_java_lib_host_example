use crate::jni_c_header::*;

pub struct Foo {
    val: i32,
}

impl Foo {
    pub fn new(val: i32) -> Self {
        Self { val }
    }

    pub fn set_field(&mut self, new_val: i32) {
        self.val = new_val;
    }

    pub fn val(&self) -> i32 {
        self.val
    }
}

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
