// dropbear-engine script template for eucalyptus
import * as dropbear from "./dropbear";

export function onLoad(s) {
    dropbear.start(s);
    // ------- Your own code here -------
    console.log("I have been awoken");


    // ----------------------------------
    // Do not remove anything outside unless
    // you know what you are doing.
    return dropbear.end();
}

export function onUpdate(s, dt: number) {
    dropbear.start(s);
    // ------- Your own code here -------
    console.log("I'm being updated!");
    

    // ----------------------------------
    // Same thing over here!
    return dropbear.end();
}