use jvm::vm::vm::VirtualMachine;

use clap::{App, Arg};
use jvm::vm::class::constant::Constant;
use jvm::vm::class::{ClassBuilder, MethodBuilder, MethodDescriptor};
use jvm::vm::linker::loader::ClassProvider;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;

fn main() {
    let app = App::new("Rust JVM Test")
        .version("0.1")
        .about("Tests the JVM in either WASM-compilation mode or interpreted mode")
        .arg(
            Arg::with_name("mode")
                .long("mode")
                .short("m")
                .help("Modes: `wasm` or `i` (interpreted)")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let mode = app.value_of("mode").unwrap();

    match mode {
        "wasm" => {
            // let mut cl = ClassLoader::new(javaroot);
            //
            // let (_, class) = cl.load_and_link_class("Main").ok().unwrap();
            //
            // println!("Creating WASM");
            //
            // let mut wasm = WasmEmitter::new(&cl, "Main!main!([Ljava/lang/String;)V");
            //
            // wasm.process_classes();
            //
            // let (wasm, memory) = wasm.build();
            //
            // fs::write("./jvm_test/wasm_out/out.wasm", &wasm);
            //
            // {
            //     let store = Store::default();
            //
            //     let module = Module::from_binary(store.engine(), &wasm[..]).unwrap();
            //
            //     let get_time = Func::wrap(&store, || {
            //         return SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
            //     });
            //
            //     let print_long = Func::wrap(&store, |x: i64| {
            //         println!("Printed long {}", x);
            //     });
            //
            //     let print_int = Func::wrap(&store, |x: i32| {
            //         println!("{}", x);
            //     });
            //
            //     let print_string = Func::wrap(&store,|caller: Caller, x: i32| {
            //         let memory = caller.get_export("heap").unwrap().into_memory().unwrap();
            //
            //         unsafe {
            //             let data = memory.data_unchecked();
            //
            //             let str = String::from_utf8(Vec::from(&data[4..100])).unwrap();
            //
            //             println!("{}", str);
            //
            //             let u = x as usize;
            //             let classpath_utf8_ptr = LittleEndian::read_u32(&data[u..u+4]) as usize;
            //             let classpath_utf8_len = LittleEndian::read_u32(&data[classpath_utf8_ptr..classpath_utf8_ptr+4]);
            //
            //             let char_arr_ptr = LittleEndian::read_u32(&data[u+4..u+8]) as usize;
            //             let char_arr_len = LittleEndian::read_u32(&data[char_arr_ptr..char_arr_ptr+4]) as usize;
            //
            //             let chars = &data[char_arr_ptr+4..char_arr_ptr + (char_arr_len * 2) + 4];
            //
            //             println!("{}", String::from_utf8(Vec::from(chars)).unwrap());
            //         }
            //     });
            //
            //     let mut imports = vec![
            //         // print_string.into(),
            //         print_int.into()
            //     ];
            //
            //     let instance = Instance::new(&store, &module, &imports).unwrap();
            //
            //     let heap = instance.get_export("heap").unwrap().into_memory().unwrap();
            //
            //     let mem_ptr = heap.data_ptr();
            //
            //     unsafe {
            //         heap.data_unchecked_mut().write(&memory.data[..]);
            //     }
            //
            //     instance.get_export("main").unwrap().into_func().unwrap().call(
            //         &[Val::I32(100)]
            //     );
            // }
        }
        "i" => {
            run_vm();
        }
        "serialization" => {
            use jvm::vm::vm::bytecode::Bytecode;

            let mut class_builder = ClassBuilder::new(String::from("Main"), Option::None);
            let mut method_builder = MethodBuilder::new(
                String::from("main"),
                MethodDescriptor::parse("(Ljava/lang/String;)V").unwrap(),
                1,
            );

            let print_string_method_utf8 =
                class_builder.add_constant(Constant::Utf8(String::from("print_string")));
            let print_string_method_type =
                class_builder.add_constant(Constant::Utf8(String::from("(Ljava/lang/String);")));
            let print_string_method_descriptor =
                class_builder.add_constant(Constant::MethodType(print_string_method_type));

            let name_and_type = class_builder.add_constant(Constant::NameAndType(
                print_string_method_utf8,
                print_string_method_descriptor,
            ));
            let class_name_utf8 = class_builder.add_constant(Constant::Utf8(String::from("Main")));
            let class = class_builder.add_constant(Constant::Class(class_name_utf8));

            let method_constant =
                class_builder.add_constant(Constant::MethodRef(class, name_and_type));

            method_builder.set_instructions(vec![
                Bytecode::Aload_n(1),
                Bytecode::Iconst_n_m1(0),
                Bytecode::Aaload,
                Bytecode::Invokestatic(method_constant),
                Bytecode::Areturn,
            ]);

            class_builder.add_method(method_builder);

            dbg!(class_builder.serialize());
        }
        _ => panic!("Unknown execution method. Must be `wasm` or `i` (interpreted)"),
    }
}

struct HashMapClassProvider {
    map: HashMap<String, Vec<u8>>,
}

impl ClassProvider for HashMapClassProvider {
    fn get_classfile(&self, classpath: &str) -> Option<&[u8]> {
        self.map.get(classpath).map(|vec| &vec[..])
    }
}

macro_rules! insert_class {
    ($provider:ident, $name:literal) => {
        $provider.map.insert(
            $name.into(),
            Vec::from(&include_bytes!(concat!("../java/", $name, ".class"))[..]),
        );
    };
}

fn run_vm() {
    let mut class_provider = HashMapClassProvider {
        map: HashMap::new(),
    };

    insert_class!(class_provider, "Main");
    // insert_class!(class_provider, "Test");
    insert_class!(class_provider, "java/lang/Object");
    insert_class!(class_provider, "java/lang/String");

    let vm = Arc::new(RwLock::new(VirtualMachine::new(Box::new(class_provider))));


    {
        let mut vm = vm.write().unwrap();

        vm.class_loader
            .write()
            .unwrap()
            .load_and_link_class("Main").unwrap();

        vm.spawn_thread(
            "thread 1",
            "Main",
            "main",
            "([Ljava/lang/String;)V",
            vec![String::from("Hello world!")],
        )
        .unwrap();

        vm.spawn_thread(
            "thread 2",
            "Main",
            "main1",
            "([Ljava/lang/String;)V",
            vec![String::from("Hello world!")],
        )
        .unwrap();

        // let mut step_start = SystemTime::now();
    }

    let vm1 = vm.clone();

    let handle1 = thread::spawn(move || {
        let jvm = vm1.read().unwrap();
        let mut thread = jvm.threads.get("thread 1").unwrap().write().unwrap();

        while thread.get_stack_count() > 0 {
            match thread.step() {
                Ok(_) => {}
                Err(e) => panic!("The Java Virtual Machine has encountered an unrecoverable error and cannot continue.\n{:?}", e)
            }
        }
    });

    let vm2 = vm;

    let handle2 = thread::spawn(move || {
        let jvm = vm2.read().unwrap();
        let mut thread = jvm.threads.get("thread 2").unwrap().write().unwrap();

        while thread.get_stack_count() > 0 {
            match thread.step() {
                Ok(_) => {}
                Err(e) => panic!("The Java Virtual Machine has encountered an unrecoverable error and cannot continue.\n{:?}", e)
            }
        }
    });

    handle1.join().unwrap();
    handle2.join().unwrap();

    // let elapsed = SystemTime::now().duration_since(start).unwrap().as_nanos();

    // println!("\n[Execution has finished.]\nAverage step time: {}ns\nSteps: {}\nTime elapsed: {}ms\nObject count: {}", elapsed/steps, steps, SystemTime::now().duration_since(start).unwrap().as_millis(), vm.heap.read().unwrap().objects.len());
}

#[cfg(test)]
mod tests {
    use jvm::vm::class::FieldDescriptor;

    #[test]
    fn descriptors() {
        let field_descriptor_str = "Ljava/lang/String;([[[B);";
        let field_descriptor = FieldDescriptor::parse(field_descriptor_str).unwrap();
        assert_eq!(String::from(&field_descriptor), field_descriptor_str);
    }
}
