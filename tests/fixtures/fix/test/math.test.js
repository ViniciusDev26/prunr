const request = require("supertest");
const { mul } = require("../src/math");

test("mul", () => expect(mul(2, 2)).toBe(4));
request;
