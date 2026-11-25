const fs = require("node:fs");
const { add, sub } = require("./math");
const lodash = require("lodash");
require("./side-effect");
console.log(add(1, 2));
