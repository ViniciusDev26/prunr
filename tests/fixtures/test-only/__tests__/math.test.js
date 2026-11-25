const request = require("supertest");
const { add, sub } = require("../lib/math");
const { oldAdd } = require("../lib/legacy");
const { connect } = require("./helpers/db");

connect();
console.log(request, add(1, 2), sub(2, 1), oldAdd(1, 1));
