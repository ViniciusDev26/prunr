const { used } = require("@demo/shared");
const { extra } = require("@demo/shared/extra");
const _ = require("lodash");

console.log(used(), extra(), _.chunk);
