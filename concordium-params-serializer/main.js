const { Buffer } = require("buffer/");
const bodyParser = require("body-parser");
const express = require("express");
const logger = require("morgan");
const { serializeInitContractParameters, serializeUpdateContractParameters } = require("@concordium/common-sdk");

const hostname = "0.0.0.0";
const port = 7433;

const app = express();
app.use(bodyParser.json()); // requires header 'Content-Type: application/json' to be set
app.use(logger("dev"));

function parseQueryParams(req, usesReceiveFunctionName) {
  const { schema, contract_name, receive_function_name, schema_version } = req.query;
  if (!schema) {
    throw new Error("missing parameter 'schema'");
  }
  const schemaBuf = Buffer.from(schema, "base64");
  if (schemaBuf.toString("base64") !== schema) {
    throw new Error("parameter 'schema' is not valid base64 - did you remember to URL encode it?");
  }
  if (usesReceiveFunctionName && !receive_function_name) {
    throw new Error("missing parameter 'receive_function_name'");
  }
  if (!usesReceiveFunctionName && receive_function_name) {
    throw new Error("unexpected parameter 'receive_function_name'");
  }
  return {
    schema: schemaBuf,
    contractName: contract_name,
    receiveFunctionName: receive_function_name,
    schemaVersion: schema_version ? Number.parseInt(schema_version) : undefined,
    verboseErrorMessage: true,
  };
}

app.post("/init", (req, res) => {
  const parameters = req.body;
  const { schema, contractName, schemaVersion, verboseErrorMessage } = parseQueryParams(req, false);
  const buf = serializeInitContractParameters(contractName, parameters, schema, schemaVersion, verboseErrorMessage);
  res.setHeader("Content-Type", "text/plain");
  res.write(buf.toString("hex"));
  res.end("\n");
});

app.post("/update", (req, res) => {
  const parameters = req.body;
  const { schema, contractName, receiveFunctionName, schemaVersion, verboseErrorMessage } = parseQueryParams(req, true);
  const buf = serializeUpdateContractParameters(contractName, receiveFunctionName, parameters, schema, schemaVersion, verboseErrorMessage);
  res.setHeader("Content-Type", "text/plain");
  res.write(buf.toString("hex"));
  res.end("\n");
});

app.listen(port, hostname, () => {
  console.log(`Listening on port ${port}.`);
});
