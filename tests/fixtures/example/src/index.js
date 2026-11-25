const express = require("express");
const dotenv = require("dotenv");
const { createInvoice } = require("./invoices");
const { Money } = require("./money");

dotenv.config();
const app = express();
app.post("/invoices", (req, res) => res.json(createInvoice(Money.of(req.body.amount))));
require(process.env.PLUGIN);
