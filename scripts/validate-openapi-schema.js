#!/usr/bin/env node

const fs = require("node:fs");

async function readInput(source) {
  if (!source || source === "-") {
    return fs.readFileSync(0, "utf8");
  }
  if (/^https?:\/\//.test(source)) {
    const response = await fetch(source);
    if (!response.ok) {
      throw new Error(`failed to fetch ${source}: ${response.status} ${response.statusText}`);
    }
    return response.text();
  }
  return fs.readFileSync(source, "utf8");
}

function parseDocument(raw) {
  try {
    return JSON.parse(raw);
  } catch (error) {
    throw new Error(`OpenAPI document must be JSON: ${error.message}`);
  }
}

function assertObject(value, path, errors) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    errors.push(`${path} must be an object`);
    return false;
  }
  return true;
}

function validateReference(document, reference, path, errors) {
  if (typeof reference !== "string") {
    errors.push(`${path}.$ref must be a string`);
    return;
  }
  if (!reference.startsWith("#/")) {
    return;
  }
  let value = document;
  for (const segment of reference.slice(2).split("/")) {
    const key = segment.replace(/~1/g, "/").replace(/~0/g, "~");
    value = value && value[key];
  }
  if (value === undefined) {
    errors.push(`${path}.$ref points to missing target ${reference}`);
  }
}

function walkSchema(document, schema, path, errors) {
  if (!assertObject(schema, path, errors)) {
    return;
  }
  if (schema.$ref) {
    validateReference(document, schema.$ref, path, errors);
  }
  for (const key of ["properties", "patternProperties", "$defs", "definitions"]) {
    if (schema[key] && assertObject(schema[key], `${path}.${key}`, errors)) {
      for (const [name, child] of Object.entries(schema[key])) {
        walkSchema(document, child, `${path}.${key}.${name}`, errors);
      }
    }
  }
  for (const key of ["items", "additionalProperties", "not"]) {
    if (schema[key] && typeof schema[key] === "object") {
      walkSchema(document, schema[key], `${path}.${key}`, errors);
    }
  }
  for (const key of ["allOf", "anyOf", "oneOf"]) {
    if (schema[key]) {
      if (!Array.isArray(schema[key])) {
        errors.push(`${path}.${key} must be an array`);
        continue;
      }
      schema[key].forEach((child, index) => walkSchema(document, child, `${path}.${key}[${index}]`, errors));
    }
  }
}

function validateOperation(document, operation, path, errors) {
  if (!assertObject(operation, path, errors)) {
    return;
  }
  if (!operation.responses || !assertObject(operation.responses, `${path}.responses`, errors)) {
    return;
  }
  for (const [status, response] of Object.entries(operation.responses)) {
    const responsePath = `${path}.responses.${status}`;
    if (response.$ref) {
      validateReference(document, response.$ref, responsePath, errors);
      continue;
    }
    if (!assertObject(response, responsePath, errors)) {
      continue;
    }
    if (!("description" in response)) {
      errors.push(`${responsePath}.description is required`);
    }
    validateContent(document, response.content, `${responsePath}.content`, errors);
  }
  for (const [index, parameter] of Object.entries(operation.parameters || [])) {
    const parameterPath = `${path}.parameters[${index}]`;
    if (parameter.$ref) {
      validateReference(document, parameter.$ref, parameterPath, errors);
      continue;
    }
    if (!assertObject(parameter, parameterPath, errors)) {
      continue;
    }
    for (const key of ["name", "in"]) {
      if (typeof parameter[key] !== "string") {
        errors.push(`${parameterPath}.${key} is required`);
      }
    }
    if (parameter.schema) {
      walkSchema(document, parameter.schema, `${parameterPath}.schema`, errors);
    }
  }
  if (operation.requestBody) {
    const requestBodyPath = `${path}.requestBody`;
    if (operation.requestBody.$ref) {
      validateReference(document, operation.requestBody.$ref, requestBodyPath, errors);
    } else if (assertObject(operation.requestBody, requestBodyPath, errors)) {
      validateContent(document, operation.requestBody.content, `${requestBodyPath}.content`, errors);
    }
  }
}

function validateContent(document, content, path, errors) {
  if (!content) {
    return;
  }
  if (!assertObject(content, path, errors)) {
    return;
  }
  for (const [contentType, mediaType] of Object.entries(content)) {
    const mediaTypePath = `${path}.${contentType}`;
    if (!assertObject(mediaType, mediaTypePath, errors)) {
      continue;
    }
    if (mediaType.schema) {
      walkSchema(document, mediaType.schema, `${mediaTypePath}.schema`, errors);
    }
  }
}

function validateOpenApi(document) {
  const errors = [];
  if (!assertObject(document, "document", errors)) {
    return errors;
  }
  if (typeof document.openapi !== "string" || !/^3\./.test(document.openapi)) {
    errors.push("document.openapi must be an OpenAPI 3.x version string");
  }
  if (!assertObject(document.info, "document.info", errors)) {
    return errors;
  }
  for (const key of ["title", "version"]) {
    if (typeof document.info[key] !== "string" || document.info[key].trim() === "") {
      errors.push(`document.info.${key} is required`);
    }
  }
  if (!assertObject(document.paths, "document.paths", errors)) {
    return errors;
  }
  const methods = new Set(["get", "put", "post", "delete", "options", "head", "patch", "trace"]);
  for (const [route, item] of Object.entries(document.paths)) {
    if (!route.startsWith("/")) {
      errors.push(`paths key ${route} must start with /`);
    }
    if (!assertObject(item, `document.paths.${route}`, errors)) {
      continue;
    }
    for (const [method, operation] of Object.entries(item)) {
      if (methods.has(method)) {
        validateOperation(document, operation, `document.paths.${route}.${method}`, errors);
      }
    }
  }
  if (document.components && assertObject(document.components, "document.components", errors)) {
    for (const [kind, entries] of Object.entries(document.components)) {
      if (!assertObject(entries, `document.components.${kind}`, errors)) {
        continue;
      }
      if (kind === "schemas") {
        for (const [name, schema] of Object.entries(entries)) {
          walkSchema(document, schema, `document.components.schemas.${name}`, errors);
        }
      }
    }
  }
  return errors;
}

async function main() {
  const source = process.argv[2];
  if (process.argv.includes("--help")) {
    console.log("usage: node scripts/validate-openapi-schema.js <openapi.json|url|->");
    return;
  }
  const raw = await readInput(source);
  const document = parseDocument(raw);
  const errors = validateOpenApi(document);
  if (errors.length > 0) {
    for (const error of errors) {
      console.error(`openapi: ${error}`);
    }
    process.exit(1);
  }
  const pathCount = Object.keys(document.paths).length;
  console.log(`OpenAPI schema ok: ${document.info.title} ${document.info.version} (${pathCount} paths)`);
}

main().catch((error) => {
  console.error(error.message);
  process.exit(1);
});
