use crc16;
use std::path::Path;
use std::fs::File;

use std::default::Default;
use std::io::Read;

use xml::reader::{EventReader, XmlEvent};

use syn::spanned::Spanned;
use proc_macro2::TokenStream;

use crate::core::MavMessage;
use crate::core::MavField;
use crate::core::MavType;
use crate::core::MavEnum;
use crate::core::MavEnumEntry;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct MavProfile {
    pub includes: Vec<String>,
    pub messages: Vec<MavMessage>,
    pub enums: Vec<MavEnum>,
}

impl MavProfile {
    // /// Go over all fields in the messages, and if you encounter an enum,
    // /// update this enum with information about whether it is a bitmask, and what
    // /// is the desired width of such.
    // fn update_enums(mut self) -> Self {
    //     for msg in &self.messages {
    //         for field in &msg.fields {
    //             if let Some(ref enum_name) = field.enumtype {
    //                 // it is an enum
    //                 if let Some(ref dsp) = field.display {
    //                     // it is a bitmask
    //                     if dsp == "bitmask" {
    //                         // find the corresponding enum
    //                         for mut enm in &mut self.enums {
    //                             if enm.name == *enum_name {
    //                                 // this is the right enum
    //                                 enm.bitfield = Some(field.mavtype.rust_type());
    //                             }
    //                         }
    //                     }
    //                 }
    //             }
    //         }
    //     }
    //     self
    // }

    //TODO verify this is no longer necessary since we're supporting both mavlink1 and mavlink2
//    ///If we are not using Mavlink v2, remove messages with id's > 254
//    fn update_messages(mut self) -> Self {
//        //println!("Updating messages");
//        let msgs = self.messages.into_iter().filter(
//            |x| x.id <= 254).collect::<Vec<MavMessage>>();
//        self.messages = msgs;
//        self
//    }


    /// Generate mavlink related rust files
    /// Includes $MESSAGE_NAME.rs, common.rs
    fn emit_mavlink(&self, output_path: &Path) {
        let mut cfg = rustfmt::config::Config::default();
        cfg.set().write_mode(rustfmt::config::WriteMode::Display);

        for message in &self.messages {
            let dest_path = Path::new(output_path).join(message.name.clone().to_lowercase() + ".rs");
            let mut outf = File::create(&dest_path).unwrap();
            let msg_tokens = message.emit_rust();

            let rust_src = msg_tokens.to_string();
            println!("{}",rust_src);    
            rustfmt::format_input(rustfmt::Input::Text(rust_src), &cfg, Some(&mut outf)).unwrap();
        }

        let common_tokens = self.emit_common();
        let rust_src = common_tokens.to_string();
        println!("{}",rust_src);    
        let dest_path = Path::new(output_path).join("common.rs");
        let mut outf = File::create(&dest_path).unwrap();
        rustfmt::format_input(rustfmt::Input::Text(rust_src), &cfg, Some(&mut outf)).unwrap();
    }


    // /// Emit rust messages
    // fn emit_msgs(&self) -> Vec<Tokens> {
    //     self.messages
    //         .iter()
    //         .map(|d| d.emit_rust())
    //         .collect::<Vec<Tokens>>()
    // }

    // /// Emit rust enums
    // fn emit_enums(&self) -> Vec<Tokens> {
    //     self.enums
    //         .iter()
    //         .map(|d| {
    //             d.emit_rust()
    //         })
    //         .collect::<Vec<Tokens>>()
    // }

    /// Get list of original message names, e.g "heartbeat"
    fn emit_enum_names(&self) -> TokenStream {
        let mut field_toks = TokenStream::new();
        self.messages
            .iter()
            .for_each(|msg| {
                let name = format!("{}", msg.name);
                let name = syn::Ident::new(&name, msg.name.span());
                field_toks.extend(quote!{#name});
        });
        field_toks
    }

    /// Emit message names with "_DATA" at the end, e.g. "HEARTBEAT_DATA"
    fn emit_struct_names(&self) -> TokenStream {
        let mut field_toks = TokenStream::new();
        self.messages
            .iter()
            .for_each(|msg| {
                let name = msg.emit_struct_name();
                field_toks.extend(quote!{#name});
        });
        field_toks
    }

    /// A list of message IDs
    fn emit_msg_ids(&self) -> TokenStream {
        let mut field_toks = TokenStream::new();
        self.messages
            .iter()
            .for_each(|msg| {
                let id = msg.id;
                field_toks.extend(quote!{#id})
            });
        field_toks
    }

    /// CRC values needed for mavlink parsing
    fn emit_msg_crc(&self) -> TokenStream {
        let mut field_toks = TokenStream::new();
        self.messages
            .iter()
            .for_each(|msg| {
                let crc = extra_crc(&msg);
                field_toks.extend(quote!{#crc});
            });
        field_toks
    }

    /// Emit include directives for all message source files
    /// E.g. "include!(concat!(env!("OUT_DIR"), "/ping.rs"));"
    fn emit_struct_includes(&self) -> TokenStream {
        let mut field_toks = TokenStream::new();
        self.messages
            .iter()
            .for_each(|msg| {
                let name = format!("/{}.rs",msg.name.to_lowercase());
                field_toks.extend(quote!{
                    include!(concat!(env!("OUT_DIR"), #name));
                    });
            });
        field_toks
    }

    /// Generate tokens for common.rs
    fn emit_common(&self) -> TokenStream {
        //let msgs = self.emit_msgs();
        let includes = self.emit_struct_includes();
        let enums = self.emit_enum_names();
        let structs = self.emit_struct_names();
        //let enums = self.emit_enums();
        let msg_ids = self.emit_msg_ids();
        let msg_crc = self.emit_msg_crc();
        //let mav_message = self.emit_mav_message(enum_names.clone(), struct_names.clone());
        let mav_message_id = self.emit_mav_message_id(enums.clone(), msg_ids.clone());
        let mav_message_ser = self.emit_mav_message_serialize(enums.clone());
        let mav_message_deser = self.emit_mav_message_deserialize(enums.clone(), structs.clone(), msg_ids.clone());

        let vartype1 = TokenStream::from(quote!(u16 u32 u64 i16 i32 i64 f32 f64));
        let vartype_write = TokenStream::from(quote!(write_u16 write_u32 write_u64 write_i16 write_i32 write_i64 write_f32 write_f64));
        let vartype_read = TokenStream::from(quote!(read_u16 read_u32 read_u64 read_i16 read_i32 read_i64 read_f32 read_f64));
        let varsize = TokenStream::from(quote!(2 4 8 2 4 8 4 8));

        quote!{
            //#(#enums)* // TODO
            use arrayvec::ArrayVec;
            use byteorder::{ByteOrder, LittleEndian};

            #includes


            #[derive(Clone, PartialEq, Debug)]
            pub enum MavMessage {
                #(#enums(#structs)),*
            }

            impl MavMessage {
                #mav_message_ser
                #mav_message_deser
                #mav_message_id
                pub fn extra_crc(id: u32) -> u8 {
                    match id {
                        #(#msg_ids => #msg_crc,)*
                        _ => 0,
                    }
                }
            }

            #[derive(Debug)]
            pub enum Error {
                NotEnoughBytes,
                UnknownMsgId,
            }

            #[doc="Trait for serialization and deserialization of primitive types"]
            pub trait MavCore where Self : Sized {
                #[doc="Serialize into a vector of bytes"]
                #[doc="If OK returns the number of bytes used from the output buffer"]
                fn ser(&self, output: &mut [u8]) -> Result<usize, Error>;
                #[doc="Deserialize from a byte vector. Return None if deserializaion failed"]
                fn deser(input: &[u8]) -> Result<Self, Error>;
                #[doc="Size of a single element in bytes"]
                fn element_size() -> usize;
            }

            #[doc="Trait for serialization and deserialization of ArrayVec types"]
            pub trait MavArray where Self : Sized {
                #[doc="Serialize into a vector of bytes"]
                #[doc="If OK returns the number of bytes used from the output buffer"]
                fn ser(&self, output: &mut [u8]) -> Result<usize, Error>;
                #[doc="Deserialize from a byte vector. Return None if deserializaion failed"]
                fn deser(input: &[u8]) -> Result<Self, Error>;
                #[doc="Size of a single array element in bytes"]
                fn element_size() -> usize;
            }

            impl MavCore for char {
                fn element_size() -> usize {
                    1
                }
                fn ser(&self, output: &mut [u8]) -> Result<usize, Error> {
                    if output.len() < Self::element_size() {
                        return Err(Error::NotEnoughBytes);
                    }
                    output[0] = *self as u8;
                    Ok(1)
                }
                fn deser(input: &[u8]) -> Result<Self, Error> {
                    if input.len() < Self::element_size() {
                        return Err(Error::NotEnoughBytes);
                    }
                    Ok(input[0] as char)
                }
            }

            impl MavCore for u8 {
                fn element_size() -> usize {
                    1
                }
                fn ser(&self, output: &mut [u8]) -> Result<usize, Error> {
                    if output.len() < Self::element_size() {
                        return Err(Error::NotEnoughBytes);
                    }
                    output[0] = *self;
                    Ok(1)
                }
                fn deser(input: &[u8]) -> Result<Self, Error> {
                    if input.len() < Self::element_size() {
                        return Err(Error::NotEnoughBytes);
                    }
                    Ok(input[0])
                }
            }

            impl MavCore for i8 {
                fn element_size() -> usize {
                    1
                }
                fn ser(&self, output: &mut [u8]) -> Result<usize, Error> {
                    if output.len() < Self::element_size() {
                        return Err(Error::NotEnoughBytes);
                    }
                    output[0] = *self as u8;
                    Ok(1)
                }
                fn deser(input: &[u8]) -> Result<Self, Error> {
                    if input.len() < Self::element_size() {
                        return Err(Error::NotEnoughBytes);
                    }
                    Ok(input[0] as i8)
                }
            }

            #(impl MavCore for #vartype1 {
                fn element_size() -> usize {
                    #varsize
                }
                fn ser(&self, output: &mut [u8]) -> Result<usize, Error> {
                    if output.len() < Self::element_size() {
                        return Err(Error::NotEnoughBytes);
                    }
                    LittleEndian:: #vartype_write (output, *self);
                    Ok(Self::element_size())
                }
                fn deser(input: &[u8]) -> Result<Self, Error> {
                    if input.len() < Self::element_size() {
                        return Err(Error::NotEnoughBytes);
                    }
                    Ok(LittleEndian:: #vartype_read (input))
                }
            })*

            impl<A: arrayvec::Array> MavArray for ArrayVec<A> where A::Item: MavCore {
                fn element_size() -> usize {
                    A::Item::element_size()
                }
                fn ser(&self, output: &mut [u8]) -> Result<usize, Error> {
                    if output.len() < ( self.len() * Self::element_size() ) {
                        return Err(Error::NotEnoughBytes);
                    }
                    let mut idx = 0;
                    for elem in self {
                        idx += elem.ser(&mut output[idx..])?;
                    }
                    Ok(idx)
                }
                fn deser(input: &[u8]) -> Result<Self, Error> {
                    let elem_len = Self::element_size();
                    if input.len() < elem_len {
                        return Err(Error::NotEnoughBytes);
                    }
                    let mut v = Self::new();
                    let items = input.len() / elem_len;
                    for item in 0..items {
                        let idx = item*elem_len;
                        v.push(A::Item::deser(&input[idx..])?);
                    }
                    Ok(v)
                }
            }
        }
    }

    // fn emit_mav_message(&self, enums: Vec<Tokens>, structs: Vec<Tokens>) -> Tokens {
    //     quote!{
    //             pub enum MavMessage {
    //                 #(#enums(#structs)),*
    //             }
    //     }
    // }

    // fn emit_mav_message_parse(
    //     &self,
    //     enums: Vec<Tokens>,
    //     structs: Vec<Tokens>,
    //     ids: Vec<Tokens>,
    // ) -> Tokens {
    //     let id_width = Ident::from("u32");
    //     quote!{
    //         pub fn parse(version: MavlinkVersion, id: #id_width, payload: &[u8]) -> Option<MavMessage> {
    //             match id {
    //                 #(#ids => Some(MavMessage::#enums(#structs::deser(version, payload).unwrap())),)*
    //                 _ => None,
    //             }
    //         }
    //     }
    // }


    /// Emit deserializing code for MavMessage enum
    fn emit_mav_message_deserialize(&self, enums: TokenStream, structs: TokenStream, ids: TokenStream) -> TokenStream {
        quote!{
            #[doc="Deserialize MavMessage"]
            pub fn deser(id: u32, payload: &[u8]) -> Result<Self, Error> {
                match id {
                    //#(&MavMessage::#enums(ref body) => body.ser(),)*
                    #(#ids => { let val = #structs::deser(payload)?; 
                            Ok(MavMessage::#enums(val))
                    })*
                    //#(#ids => MavMessage::#enums(#structs::deser(payload)),)*
                    _ => Err(Error::UnknownMsgId),
                }
            }
        }
    }

    /// Emits `pub fn message_id(&self) -> u32` which retuns message ID corresponding
    /// to each `MavMessage` enum variant
    fn emit_mav_message_id(&self, enums: TokenStream, ids: TokenStream) -> TokenStream {
        quote!{
            pub fn message_id(&self) -> u32 {
                match self {
                    #(MavMessage::#enums(..) => #ids,)*
                }
            }
        }
    }

    /// Emit serializing code for MavMessage enum
    fn emit_mav_message_serialize(&self, enums: TokenStream) -> TokenStream {
        quote!{
            #[doc="Serialize MavMessage"]
            pub fn ser(&self) -> Result<Vec<u8>, Error> {
                match self {
                    #(&MavMessage::#enums(ref body) => body.ser(),)*
                }
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum MavXmlElement {
    Version,
    Mavlink,
    Include,
    Enums,
    Enum,
    Entry,
    Description,
    Param,
    Messages,
    Message,
    Field,
    Deprecated,
    Wip,
    Extensions,
}

fn identify_element(s: &str) -> Option<MavXmlElement> {
    use self::MavXmlElement::*;
    match s {
        "version" => Some(Version),
        "mavlink" => Some(Mavlink),
        "include" => Some(Include),
        "enums" => Some(Enums),
        "enum" => Some(Enum),
        "entry" => Some(Entry),
        "description" => Some(Description),
        "param" => Some(Param),
        "messages" => Some(Messages),
        "message" => Some(Message),
        "field" => Some(Field),
        "deprecated" => Some(Deprecated),
        "wip" => Some(Wip),
        "extensions" => Some(Extensions),
        _ => None,
    }
}

fn is_valid_parent(p: Option<MavXmlElement>, s: MavXmlElement) -> bool {
    use self::MavXmlElement::*;
    match s {
        Version => p == Some(Mavlink),
        Mavlink => p == None,
        Include => p == Some(Mavlink),
        Enums => p == Some(Mavlink),
        Enum => p == Some(Enums),
        Entry => p == Some(Enum),
        Description => p == Some(Entry) || p == Some(Message) || p == Some(Enum),
        Param => p == Some(Entry),
        Messages => p == Some(Mavlink),
        Message => p == Some(Messages),
        Field => p == Some(Message),
        Deprecated => p == Some(Entry) || p == Some(Message) || p == Some(Enum),
        Wip => p == Some(Entry) || p == Some(Message) || p == Some(Enum),
        Extensions => p == Some(Message),
    }
}


pub fn parse_profile(file: &mut Read) -> MavProfile {
    let mut stack: Vec<MavXmlElement> = vec![];

    let mut profile = MavProfile {
        includes: vec![],
        messages: vec![],
        enums: vec![],
    };

    let mut field = MavField::default();
    let mut message = MavMessage::default();
    let mut mavenum = MavEnum::default();
    let mut entry = MavEnumEntry::default();
    let mut paramid: Option<usize> = None;

    let parser = EventReader::new(file);
    for e in parser {
        match e {
            Ok(XmlEvent::StartElement {
                name,
                attributes: attrs,
                ..
            }) => {
                let id = match identify_element(&name.to_string()) {
                    None => {
                        panic!("unexpected element {:?}", name);
                    }
                    Some(kind) => kind,
                };

                if !is_valid_parent(
                    match stack.last().clone() {
                        Some(arg) => Some(arg.clone()),
                        None => None,
                    },
                    id.clone(),
                ) {
                    panic!("not valid parent {:?} of {:?}", stack.last(), id);
                }

                match id {
                    MavXmlElement::Message => {
                        message = Default::default();
                    }
                    MavXmlElement::Field => {
                        field = Default::default();
                    }
                    MavXmlElement::Enum => {
                        mavenum = Default::default();
                    }
                    MavXmlElement::Entry => {
                        entry = Default::default();
                    }
                    MavXmlElement::Param => {
                        paramid = None;
                    }
                    _ => (),
                }

                stack.push(id);

                for attr in attrs {
                    match stack.last() {
                        Some(&MavXmlElement::Enum) => match attr.name.local_name.clone().as_ref() {
                            "name" => {
                                mavenum.name =
                                    attr.value
                                        .clone()
                                        .split("_")
                                        .map(|x| x.to_lowercase())
                                        .map(|x| {
                                            let mut v: Vec<char> = x.chars().collect();
                                            v[0] = v[0].to_uppercase().nth(0).unwrap();
                                            v.into_iter().collect()
                                        })
                                        .collect::<Vec<String>>()
                                        .join("");
                                //mavenum.name = attr.value.clone();
                            }
                            _ => (),
                        },
                        Some(&MavXmlElement::Entry) => {
                            match attr.name.local_name.clone().as_ref() {
                                "name" => {
                                    entry.name = attr.value.clone();
                                }
                                "value" => {
                                    entry.value = Some(attr.value.parse::<i32>().unwrap());
                                }
                                _ => (),
                            }
                        }
                        Some(&MavXmlElement::Message) => {
                            match attr.name.local_name.clone().as_ref() {
                                "name" => {
                                    /*message.name = attr
                                        .value
                                        .clone()
                                        .split("_")
                                        .map(|x| x.to_lowercase())
                                        .map(|x| {
                                            let mut v: Vec<char> = x.chars().collect();
                                            v[0] = v[0].to_uppercase().nth(0).unwrap();
                                            v.into_iter().collect()
                                        })
                                        .collect::<Vec<String>>()
                                        .join("");
                                        */
                                    message.name = attr.value.clone();
                                }
                                "id" => {
                                    //message.id = attr.value.parse::<u8>().unwrap();
                                    message.id = attr.value.parse::<u32>().unwrap();
                                }
                                _ => (),
                            }
                        }
                        Some(&MavXmlElement::Field) => {
                            match attr.name.local_name.clone().as_ref() {
                                "name" => {
                                    field.name = attr.value.clone();
                                    if field.name == "type" {
                                        field.name = "mavtype".to_string();
                                    }
                                }
                                "type" => {
                                    field.mavtype = MavType::parse_type(&attr.value).unwrap();
                                }
                                "enum" => {
                                    field.enumtype = Some(
                                        attr.value
                                            .clone()
                                            .split("_")
                                            .map(|x| x.to_lowercase())
                                            .map(|x| {
                                                let mut v: Vec<char> = x.chars().collect();
                                                v[0] = v[0].to_uppercase().nth(0).unwrap();
                                                v.into_iter().collect()
                                            })
                                            .collect::<Vec<String>>()
                                            .join(""),
                                    );
                                    //field.enumtype = Some(attr.value.clone());
                                }
                                "display" => {
                                    field.display = Some(attr.value);
                                }
                                _ => (),
                            }
                        }
                        Some(&MavXmlElement::Param) => {
                            if let None = entry.params {
                                entry.params = Some(vec![]);
                            }
                            match attr.name.local_name.clone().as_ref() {
                                "index" => {
                                    paramid = Some(attr.value.parse::<usize>().unwrap());
                                }
                                _ => (),
                            }
                        }
                        _ => (),
                    }
                }
            }
            Ok(XmlEvent::Characters(s)) => {
                use self::MavXmlElement::*;
                match (stack.last(), stack.get(stack.len() - 2)) {
                    (Some(&Description), Some(&Message)) => {
                        message.description = Some(s.replace("\n", " "));
                    }
                    (Some(&Field), Some(&Message)) => {
                        field.description = Some(s.replace("\n", " "));
                    }
                    (Some(&Description), Some(&Enum)) => {
                        mavenum.description = Some(s.replace("\n", " "));
                    }
                    (Some(&Description), Some(&Entry)) => {
                        entry.description = Some(s.replace("\n", " "));
                    }
                    (Some(&Param), Some(&Entry)) => {
                        if let Some(ref mut params) = entry.params {
                            params.insert(paramid.unwrap() - 1, s);
                        }
                    }
                    (Some(&Include), Some(&Mavlink)) => {
                        println!("TODO: include {:?}", s);
                    }
                    (Some(&Version), Some(&Mavlink)) => {
                        println!("TODO: version {:?}", s);
                    }
                    (Some(Deprecated), _) => {
                        println!("TODO: deprecated {:?}", s);
                    }
                    data => {
                        panic!("unexpected text data {:?} reading {:?}", data, s);
                    }
                }
            }
            Ok(XmlEvent::EndElement { .. }) => {
                match stack.last() {
                    Some(&MavXmlElement::Field) => message.fields.push(field.clone()),
                    Some(&MavXmlElement::Entry) => {
                        mavenum.entries.push(entry.clone());
                    }
                    Some(&MavXmlElement::Message) => {
                        // println!("message: {:?}", message);
                        let mut msg = message.clone();
                        msg.fields.sort_by(|a, b| a.mavtype.compare(&b.mavtype));
                        profile.messages.push(msg);
                    }
                    Some(&MavXmlElement::Enum) => {
                        profile.enums.push(mavenum.clone());
                    }
                    _ => (),
                }
                stack.pop();
                // println!("{}-{}", indent(depth), name);
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
            _ => {}
        }
    }

    //let profile = profile.update_messages(); //TODO verify no longer needed
    //profile.update_enums()
    profile.messages = profile.messages[1..10].to_vec();
    profile
}

/// Generate protobuf represenation of mavlink message set
/// Generate rust representation of mavlink message set with appropriate conversion methods
pub fn generate<R: Read>(input: &mut R, output_path: &Path) {
    let profile = parse_profile(input);

    profile.emit_mavlink(output_path);
    // // rust file
    // let rust_tokens = profile.emit_rust();
    // //writeln!(output_rust, "{}", rust_tokens).unwrap();

    // let rust_src = rust_tokens.into_string();
    // let mut cfg = rustfmt::config::Config::default();
    // cfg.set().write_mode(rustfmt::config::WriteMode::Display);
    // rustfmt::format_input(rustfmt::Input::Text(rust_src), &cfg, Some(output_rust)).unwrap();
}

/// CRC operates over names of the message and names of its fields
/// Hence we have to preserve the original uppercase names delimited with an underscore
/// For field names, we replace "type" with "mavtype" to make it rust compatible (this is
/// needed for generating sensible rust code), but for calculating crc function we have to
/// use the original name "type"
pub fn extra_crc(msg: &MavMessage) -> u8 {
    // calculate a 8-bit checksum of the key fields of a message, so we
    // can detect incompatible XML changes
    let mut crc = crc16::State::<crc16::MCRF4XX>::new();
    crc.update(msg.name.as_bytes());
    crc.update(" ".as_bytes());

    let mut f = msg.fields.clone();
    f.sort_by(|a, b| a.mavtype.compare(&b.mavtype));
    for field in &f {
        crc.update(field.mavtype.primitive_type().as_bytes());
        crc.update(" ".as_bytes());
        if field.name == "mavtype" {
            crc.update("type".as_bytes());
        } else {
            crc.update(field.name.as_bytes());
        }
        crc.update(" ".as_bytes());
        if let MavType::Array(_, size) = field.mavtype {
            crc.update(&[size as u8]);
        }
    }

    let crcval = crc.get();
    ((crcval & 0xFF) ^ (crcval >> 8)) as u8
}
