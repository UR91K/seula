import { render } from "solid-js/web";
import "../../../mockup/colors.css";
import "../../../mockup/shell.css";
import "../../../mockup/components.css";
import "../../../mockup/collections.css";
import "../../../mockup/plugins.css";
import "../../../mockup/samples.css";
import "../../../mockup/stats.css";
import "../../shared/app.css";
import { App } from "./App";

render(() => <App />, document.getElementById("app")!);
