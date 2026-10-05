import { Show, createEffect, createSignal, on } from "solid-js";
import type { CollectionRow } from "../../../shared/types";
import { Icon } from "../shell/parts";
import { api } from "../shell/shell";

/** A collection's cover, or the album glyph where there is none, or where the image does
 *  not load (the daemon answers 404 for a file it no longer has). */
export function Cover(props: { c: Pick<CollectionRow, "cover_art_id">; class?: string }) {
  const [failed, setFailed] = createSignal(false);
  // A new cover id is a new image: try it.
  createEffect(on(() => props.c.cover_art_id, () => setFailed(false)));
  return (
    <Show when={props.c.cover_art_id && !failed()} fallback={<span class={`noart ${props.class ?? ""}`}><Icon name="album" /></span>}>
      <img class={props.class} src={api.mediaUrl(props.c.cover_art_id!)} alt="" onError={() => setFailed(true)} />
    </Show>
  );
}
