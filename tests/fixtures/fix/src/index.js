const { add, unusedHelper } = require("./math");
const fs = require("fs");
const legacy = require("./legacy");
const _ = require("lodash");
const { greet, stillUsed } = require("./greet.mjs");

console.log(add(1, 2), _.chunk, greet(), stillUsed);
