using Workerd = import "/workerd/workerd.capnp";

    const helloWorldExample :Workerd.Config = (
      services = [ (name = "main", worker = .helloWorld) ],
      sockets = [ ( name = "http", address = "*:34389", http = (), service = "main" ) ]
    );
    
    const helloWorld :Workerd.Worker = (
      serviceWorkerScript = embed "0x69096258e01629bbc8625cf4fa8644eefb9095346658a26cd6924579888c7cf0caa2c643-f3ed-4695-81a0-c642f87dfac0.js",
      compatibilityDate = "2022-09-16",
    );