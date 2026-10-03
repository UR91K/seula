import { mount } from "svelte";
import "../../../mockup/colors.css";
import "../../../mockup/shell.css";
import "../../../mockup/components.css";
import "../../shared/app.css";
import App from "./App.svelte";

mount(App, { target: document.getElementById("app")! });
