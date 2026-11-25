const _ = require("lodash");

exports.used = () => _.identity(1);
exports.unused = () => 2;
