CREATE TABLE public.channel_thread_reads (
    channel_id integer NOT NULL REFERENCES public.channels(id) ON DELETE CASCADE,
    root_message_id integer NOT NULL REFERENCES public.channel_messages(id) ON DELETE CASCADE,
    user_id integer NOT NULL REFERENCES public.users(id) ON DELETE CASCADE,
    message_id integer NOT NULL REFERENCES public.channel_messages(id) ON DELETE CASCADE,
    updated_at timestamp without time zone DEFAULT now() NOT NULL,
    PRIMARY KEY (channel_id, root_message_id, user_id)
);

CREATE INDEX index_channel_thread_reads_on_message_id
    ON public.channel_thread_reads USING btree (message_id);
