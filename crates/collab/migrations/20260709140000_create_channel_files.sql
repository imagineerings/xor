CREATE TABLE public.channel_files (
    id uuid PRIMARY KEY,
    channel_id integer NOT NULL REFERENCES public.channels(id) ON DELETE CASCADE,
    message_id integer REFERENCES public.channel_messages(id) ON DELETE SET NULL,
    filename text NOT NULL,
    file_size bigint NOT NULL,
    mime_type text NOT NULL,
    storage_path text NOT NULL,
    uploader_id integer NOT NULL REFERENCES public.users(id) ON DELETE CASCADE,
    image_width integer,
    image_height integer,
    duration_ms bigint,
    created_at timestamp without time zone DEFAULT now() NOT NULL,
    uploaded_at timestamp without time zone
);

CREATE INDEX index_channel_files_on_channel_id_and_created_at
    ON public.channel_files USING btree (channel_id, created_at);

CREATE INDEX index_channel_files_on_message_id
    ON public.channel_files USING btree (message_id);

CREATE INDEX index_channel_files_on_uploader_id
    ON public.channel_files USING btree (uploader_id);
