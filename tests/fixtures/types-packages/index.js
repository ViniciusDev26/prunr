const fs = require("fs");
const lib = require("@scope/lib");
const typedOnly = require("@scope/typed-only");

module.exports = () => [fs, lib, typedOnly];
