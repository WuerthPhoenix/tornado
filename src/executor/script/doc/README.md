# Script Executor

An executor that runs custom shell scripts.

## How it Works

To be correctly processed by this executor, an Action should provide in its payload 
two entries; first, a script path on the local filesystem of the executor process and, second, 
all the parameters required to the resolve the script placeholders.

The script path is identified by the payload key __script__; it is important to verify that the 
executor has the rights to read and execute it.

Additionally, if the scripts has placeholders, then the payload should contain a key for each of them.

For example, a valid Action is:
```json
{
    "id": "script",
    "payload" : {
        "script": "./usr/script/my_script.sh ${arg_one} ${arg_two}",
        "arg_one": "tornado",
        "arg_two": "rust"
    }
}
```

In this case the executor will launch the script _my_script.sh_ replacing 
_${arg_one}_ with "tornado" and _${arg_two}_ with "rust". Consequently, the resulting 
command will be:
```bash
./usr/script/my_script.sh tornado rust
```


Other action examples are:

- Non valid because missing "arg_two" entry in the payload: 
```json
{
    "id": "script",
    "payload" : {
        "script": "./usr/script/my_script.sh ${arg_one} ${arg_two}",
        "arg_one": "tornado"
    }
}
```

- Non valid because missing "script" entry in the payload: 
```json
{
    "id": "script",
    "payload" : {
        "arg_one": "tornado"
    }
}
```

- Valid because the script has not placeholders: 
```json
{
    "id": "script",
    "payload" : {
        "script": "./usr/script/my_script.sh"
    }
}
```