// dropbear-engine script template for eucalyptus
import * as dropbear from "./dropbear.ts";

export function onLoad(e) {
    const entity = new dropbear.Entity(e);
    // ------- Your own code here -------
    console.log("I have been awoken");


    // ----------------------------------
    // Do not remove anything outside unless
    // you know what you are doing.
    return entity.toEntityData();
}

export function onUpdate(e, dt: number) {
    const entity = new dropbear.Entity(e);
    // ------- Your own code here -------
    console.log("I'm being updated!");


    // ----------------------------------
    // Same thing over here!
    return entity.toEntityData();
}
