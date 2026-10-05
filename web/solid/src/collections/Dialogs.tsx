// The four collection dialogs (ADR-0044) on the shared Dialog (ADR-0056). A dialog stays
// open while its write is out, and closes only when the write succeeds; a failure goes to
// the status bar and leaves what was typed.

import { Show, createSignal, untrack, type JSX } from "solid-js";
import { plural } from "../../../shared/projects";
import type { CollectionRow } from "../../../shared/types";
import { Dialog } from "../shell/Dialog";
import { Icon } from "../shell/parts";
import { Cover } from "./parts";
import { setNotice } from "../shell/shell";
import {
  closeDialog, createCollection, cui, deleteCollection, duplicateCollection, editCollection, removeCover,
  selectedCollection, setCover,
} from "./state";

/** Run a dialog's write, close on success. The submit button is disabled while it runs. */
function useSubmit(write: () => Promise<boolean>) {
  const [busy, setBusy] = createSignal(false);
  const submit = async () => {
    if (busy()) return;
    setBusy(true);
    try { if (await write()) closeDialog(); } finally { setBusy(false); }
  };
  return { busy, submit };
}

const Buttons = (props: { label: string; kind?: "primary" | "danger"; disabled: boolean; onSubmit(): void }) => (
  <>
    <button class="btn" onClick={closeDialog}>Cancel</button>
    <button class={`btn ${props.kind ?? "primary"}`} disabled={props.disabled} onClick={() => props.onSubmit()}>{props.label}</button>
  </>
);

/** A single-line field; Enter submits. */
function Field(props: { value: string; onInput(v: string): void; onEnter(): void }): JSX.Element {
  return (
    <label class="field focus">
      <input value={props.value} onInput={(e) => props.onInput(e.currentTarget.value)}
        onKeyDown={(e) => { if (e.key === "Enter") { e.preventDefault(); props.onEnter(); } }} />
    </label>
  );
}

function Description(props: { value: string; onInput(v: string): void }) {
  return <textarea class="notes" placeholder="What ties these together?" value={props.value}
    onInput={(e) => props.onInput(e.currentTarget.value)} />;
}

function NewDialog() {
  const [name, setName] = createSignal("");
  const [description, setDescription] = createSignal("");
  const { busy, submit } = useSubmit(() => createCollection(name(), description()));
  const ready = () => !!name().trim() && !busy();
  return (
    <Dialog title="New collection" width={160} onClose={closeDialog}
      buttons={<Buttons label="Create collection" disabled={!ready()} onSubmit={submit} />}>
      <label>Name</label>
      <Field value={name()} onInput={setName} onEnter={() => ready() && submit()} />
      <label>Description <span class="faint">(optional)</span></label>
      <Description value={description()} onInput={setDescription} />
      <p class="dim" style={{ "margin-top": "calc(var(--u) * 3)" }}>
        Add projects from the Projects view: select them, then Collection ▾ › Add to collection.
      </p>
    </Dialog>
  );
}

/** The cover, with a picker and a drop target (ADR-0058). The webview's own file input
 *  opens the native dialog; the bytes go to the daemon, which keeps its own copy. */
function CoverEdit(props: { c: CollectionRow }) {
  let input!: HTMLInputElement;
  const [over, setOver] = createSignal(false);
  const take = (file: File | undefined) => {
    if (!file) return;
    if (!file.type.startsWith("image/")) { setNotice(`${file.name} is not an image`); return; }
    setCover(props.c.id, file);
  };
  return (
    <div class="coveredit"
      onDragOver={(e) => { e.preventDefault(); setOver(true); }}
      onDragLeave={() => setOver(false)}
      onDrop={(e) => { e.preventDefault(); setOver(false); take(e.dataTransfer?.files[0]); }}>
      <div class="art" style={{ outline: over() ? "1px solid var(--accent)" : undefined }}><Cover c={props.c} /></div>
      <div>
        <div class="row">
          <input ref={input} type="file" accept="image/*" hidden
            onChange={(e) => { take(e.currentTarget.files?.[0]); e.currentTarget.value = ""; }} />
          <button class="btn" onClick={() => input.click()}>{props.c.cover_art_id ? "Change image…" : "Choose image…"}</button>
          <Show when={props.c.cover_art_id}><button class="btn" onClick={() => removeCover(props.c.id)}>Remove</button></Show>
        </div>
        <p class="faint">Or drop an image here. It shows on the card.</p>
      </div>
    </div>
  );
}

function EditDialog(props: { c: CollectionRow }) {
  // What it starts with; the dialog's own edits take over from there.
  const [name, setName] = createSignal(untrack(() => props.c.name));
  const [description, setDescription] = createSignal(untrack(() => props.c.description ?? ""));
  const { busy, submit } = useSubmit(() => editCollection(props.c.id, name(), description()));
  const ready = () => !!name().trim() && !busy();
  return (
    <Dialog title="Edit collection" width={180} onClose={closeDialog}
      buttons={<Buttons label="Save" disabled={!ready()} onSubmit={submit} />}>
      <CoverEdit c={props.c} />
      <label>Name</label>
      <Field value={name()} onInput={setName} onEnter={() => ready() && submit()} />
      <label>Description</label>
      <Description value={description()} onInput={setDescription} />
    </Dialog>
  );
}

function DuplicateDialog(props: { c: CollectionRow }) {
  const [name, setName] = createSignal(untrack(() => `${props.c.name} copy`));
  const { busy, submit } = useSubmit(() => duplicateCollection(props.c.id, name()));
  const ready = () => !!name().trim() && !busy();
  return (
    <Dialog title="Duplicate collection" width={160} onClose={closeDialog}
      buttons={<Buttons label="Duplicate" disabled={!ready()} onSubmit={submit} />}>
      <label>Name of the copy</label>
      <Field value={name()} onInput={setName} onEnter={() => ready() && submit()} />
      <p class="dim" style={{ "margin-top": "calc(var(--u) * 3)" }}>
        The copy has the same {plural(props.c.project_count, "project")} in the same order, and the same description and cover.
      </p>
    </Dialog>
  );
}

function DeleteDialog(props: { c: CollectionRow }) {
  const { busy, submit } = useSubmit(() => deleteCollection(props.c.id));
  const n = () => props.c.project_count;
  return (
    <Dialog title="Delete collection" width={160} onClose={closeDialog}
      buttons={<Buttons label="Delete collection" kind="danger" disabled={busy()} onSubmit={submit} />}>
      <div class="warn">
        <Icon name="warning" fill />
        <div>
          <p>Delete the collection <b>{props.c.name}</b>?</p>
          <p class="dim">
            {n()
              ? `Its ${plural(n(), "project")} ${n() === 1 ? "is" : "are"} not touched: ${n() === 1 ? "it stays" : "they stay"} in Seula and in any other collection.`
              : "It holds no projects."}{" "}
            Only the list and its order are deleted.
          </p>
        </div>
      </div>
    </Dialog>
  );
}

/** Whichever dialog is open. Edit, duplicate and delete act on the selected collection. */
export function Dialogs() {
  return (
    <Show when={cui.dialog}>{(kind) => (
      <Show when={kind() === "new"} fallback={
        <Show when={selectedCollection()} keyed>{(c) => (
          <>
            <Show when={kind() === "edit"}><EditDialog c={c} /></Show>
            <Show when={kind() === "duplicate"}><DuplicateDialog c={c} /></Show>
            <Show when={kind() === "delete"}><DeleteDialog c={c} /></Show>
          </>
        )}</Show>}>
        <NewDialog />
      </Show>
    )}</Show>
  );
}
