# Script Executor

An executor that runs custom shell scripts.



## How It Works

To be correctly processed by this executor, an Action should provide two entries in its payload:
the path to a script on the local filesystem of the executor process, and all the parameters
required to the resolve the script placeholders.

The script path is identified by the payload key __script__; it is important to verify that the 
executor has read and execute rights at that path. Additionally, if a script has placeholders,
then the payload should contain a key and valid value for each one.

An example of a valid Action is:
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

In this case the executor will launch the script _my_script.sh_, replacing _${arg_one}_ with
"tornado" and _${arg_two}_ with "rust". Consequently, the resulting command executed will be:
```bash
./usr/script/my_script.sh tornado rust
```

Other action examples are:

- An invalid action due to the missing "arg_two" entry in the payload: 
```json
{
    "id": "script",
    "payload" : {
        "script": "./usr/script/my_script.sh ${arg_one} ${arg_two}",
        "arg_one": "tornado"
    }
}
```

- An invalid action due to the missing "script" entry in the payload: 
```json
{
    "id": "script",
    "payload" : {
        "arg_one": "tornado"
    }
}
```

- An action that is valid since the script does not have placeholders: 
```json
{
    "id": "script",
    "payload" : {
        "script": "./usr/script/my_script.sh"
    }
}
```
