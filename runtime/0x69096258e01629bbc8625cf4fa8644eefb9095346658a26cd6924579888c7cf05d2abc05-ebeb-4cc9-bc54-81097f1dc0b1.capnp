using Workerd = import "/workerd/workerd.capnp";

    const helloWorldExample :Workerd.Config = (
      services = [ (name = "main", worker = .helloWorld) ],
      sockets = [ ( name = "http", address = "*:43261", http = (), service = "main" ) ]
    );
    
    const helloWorld :Workerd.Worker = (
      serviceWorkerScript = embed "0x69096258e01629bbc8625cf4fa8644eefb9095346658a26cd6924579888c7cf05d2abc05-ebeb-4cc9-bc54-81097f1dc0b1.js",
      compatibilityDate = "2022-09-16",
    );