const CopyWebpackPlugin = require("copy-webpack-plugin");
const path = require('path');

module.exports = {
  entry: "./bootstrap.js",
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "bootstrap.js",
  },
  mode: "development",
  plugins: [
    new CopyWebpackPlugin({
	    patterns: [{from: './index.html'}]
    })
  ],
  module: {
	  rules: [
		  {
			  test: /worker\.js$/,
			  use: { loader: "worker-loader" },
		  },
	  ],
  },
  devServer: {
    hot: true
  },
  experiments: {
    asyncWebAssembly: true
  }
};
