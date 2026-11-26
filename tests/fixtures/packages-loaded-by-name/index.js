const { ChartJSNodeCanvas } = require("chartjs-node-canvas");

// chartjs-node-canvas requires its plugins by name.
const canvas = new ChartJSNodeCanvas({
  width: 400,
  height: 300,
  plugins: {
    modern: ["chartjs-plugin-annotation"],
    requireLegacy: ["chartjs-plugin-datalabels"],
  },
});

// Another call's strings are not packages.
console.log(canvas, ["left-pad"]);
