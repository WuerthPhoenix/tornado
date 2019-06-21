# Script Executor

An executor that runs custom shell scripts on a Unix-like system.


## How It Works

To be correctly processed by this executor, an Action should provide two entries in its payload:
the path to a script on the local filesystem of the executor process, and all the arguments
to be passed to the script itself.

The script path is identified by the payload key __script__; it is important to verify that the
executor has read and execute rights at that path. 

The script arguments are identified by the payload key __args__;
if present, they are passed as command line arguments when the script
is executed.

An example of a valid Action is:
```json
{
    "id": "script",
    "payload" : {
        "script": "./usr/script/my_script.sh",
        "args": [
            "tornado",
            "rust"
        ] 
    }
}
```

In this case the executor will launch the script _my_script.sh_ with the arguments
"tornado" and "rust". Consequently, the resulting command executed will be:
```bash
./usr/script/my_script.sh tornado rust
```

## Other ways of passing the arguments
There are different ways to pass the arguments for a script:

- Passing arguments as a String:
  
  ```json
  {
    "id": "script",
    "payload" : {
        "script": "./usr/script/my_script.sh",
        "args": "arg_one arg_two -a --something else"
    }
  }
  ```

  If args is a String, the entire String is
  appended to the script. In this case the resulting command will be:
  
  ```bash
  ./usr/script/my_script.sh arg_one arg_two -a --something else 
  ```

- Passing arguments in an array:
  
  ```json
  {
    "id": "script",
    "payload" : {
        "script": "./usr/script/my_script.sh",
        "args": [
            "--arg_one tornado",
            "arg_two",
            true,
            100
        ] 
    }
  }
  ```
  
  Here we are passing arguments in an array; they are passed to the script in the exact order
  they are declared. In this case the resulting command will be:
  
  ```bash
  ./usr/script/my_script.sh --arg_one tornado arg_two true 100 
  ```

- Passing arguments in a map:
  
  ```json
  {
    "id": "script",
    "payload" : {
        "script": "./usr/script/my_script.sh",
        "args": {
          "arg_one": "tornado",
          "arg_two": "rust"
      }
    }
  }
  ```
  
  When arguments are passed in a map, each entry in the map is considered a pair of
  option key and option value. Each pair is passed to the script using the default style to 
  pass options to a Unix executable which is _--key_ followed by the _value_. 
  Consequently, the resulting command will be:
  
  ```bash
  ./usr/script/my_script.sh --arg_one tornado --arg_two rust
  ```

  Please note that ordering is not granted in this case; so the resulting command line
  could also be:
  
  ```bash
  ./usr/script/my_script.sh --arg_two rust --arg_one tornado
  ```
  
  If the order of the arguments matters, you should pass them using the String or
  the Array based approach.

- Passing no arguments:
  
  ```json
  {
    "id": "script",
    "payload" : {
        "script": "./usr/script/my_script.sh"
    }
  }
  ```
  
  Since the arguments are not mandatory, they can be omitted.
  In this case the resulting command will be a simple:
  
  ```bash
  ./usr/script/my_script.sh 
  ```
 