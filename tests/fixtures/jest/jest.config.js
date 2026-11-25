module.exports = {
  testEnvironment: "jsdom",
  setupFilesAfterEnv: ["@testing-library/jest-dom", "<rootDir>/test/setup.js"],
  reporters: ["default", "jest-junit"],
};
