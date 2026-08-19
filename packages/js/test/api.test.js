import test from "node:test";
import assert from "node:assert/strict";
import {BaeldAgent,installFiltering,withModelWait} from "../src/index.js";
test("public API is importable",()=>{assert.equal(typeof BaeldAgent.connect,"function");assert.equal(typeof installFiltering,"function");assert.equal(typeof withModelWait,"function")});
