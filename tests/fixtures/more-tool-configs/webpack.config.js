module.exports = {
  entry: { app: "./src/app", polyfills: process.env.POLYFILLS },
  module: { rules: [{ test: /\.js$/, use: [{ loader: "babel-loader" }] }] },
};
