CREATE TABLE public.channel_message_reactions (
    channel_id integer NOT NULL REFERENCES public.channels(id) ON DELETE CASCADE,
    message_id integer NOT NULL REFERENCES public.channel_messages(id) ON DELETE CASCADE,
    user_id integer NOT NULL REFERENCES public.users(id) ON DELETE CASCADE,
    emoji_name text NOT NULL,
    created_at timestamp without time zone DEFAULT now() NOT NULL,
    PRIMARY KEY (message_id, user_id, emoji_name)
);

CREATE INDEX index_channel_message_reactions_on_channel_id_and_message_id ON public.channel_message_reactions USING btree (channel_id, message_id);

CREATE INDEX index_channel_message_reactions_on_user_id ON public.channel_message_reactions USING btree (user_id);
