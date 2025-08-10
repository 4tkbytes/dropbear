# Utils

In this page, it will show you the different utility functions available. 

## log
The log function is used to (as the name suggests) log a string to the console. 
### Parameters

- `str` - The message to be printed

### Example

```rhai
log("Hello log function!");
```

## time
The time function returns an `f64` value to the current `UNIX_EPOCH` time of the computer running. This is particularly useful for stopwatches and comparing time. 

### Parameters
No params for this one!

### Example
```rhai
time() // returns the current unix_epoch time
```
