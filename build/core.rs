use std::cmp::Ordering;
use std::default::Default;

use syn::spanned::Spanned;
use proc_macro2::TokenStream;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct MavMessage {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    pub fields: Vec<MavField>,
}

impl MavMessage {
    /// Return Token of "MESSAGE_NAME_DATA
    /// for mavlink struct data
    pub fn emit_struct_name(&self) -> TokenStream {
        let name = format!("{}_DATA", self.name);
        let varname = syn::Ident::new(&name, self.name.span());
        quote!(#varname)
    }

    /// Emits names and types for each element of the message struct,
    /// e.g. "#[doc = "PING sequence."] pub seq: u32,"
    /// as well as the length of the struct in bytes (no truncation)
    fn emit_name_types(&self) -> (TokenStream, usize) {
        let mut encoded_payload_len: usize = 0;
        let mut field_toks = TokenStream::new();
        for field in &self.fields {
            let nametype = field.emit_name_type();
            encoded_payload_len +=  field.mavtype.len();
            let description = field.emit_description();

            field_toks.extend(quote!{
                    #description
                    #nametype
                });
        };
        (field_toks, encoded_payload_len)
    }

    /// Generate description for the given message
    /// #[cfg(feature = "emit-description")]
    fn emit_description(&self) -> TokenStream {
        let desc = format!("id: {}", self.id);
        let mut desc = quote!{#[doc = #desc]};

        if let Some(val) = self.description.clone() {
            let val = &format!("{}\n",val);
            desc.extend(quote!{#[doc = #val]})
        }
        desc
    }

    /// Emit tokens representing serialization of the message fields
    fn emit_serialize_vars(&self) -> TokenStream {
        let mut field_toks = TokenStream::new();
        self.fields
            .iter()
            .for_each(|field| {
                let val = field.rust_writer();
                field_toks.extend(quote!{#val});
            });
        field_toks
    }

    /// Emit tokens for deserialization of the message fields
    fn emit_deserialize_vars(&self) -> TokenStream {
        let mut field_toks = TokenStream::new();
        self.fields
            .iter()
            .for_each(|field| {
                let val = field.rust_reader();
                field_toks.extend(quote!{#val});
            });
        field_toks
    }

    // fn emit_deserialize_vars(&self) -> Tokens {
    //     let deser_vars = self.fields.iter()
    //         .map(|f| {
    //             f.rust_reader()
    //         }).collect::<Vec<Tokens>>();

    //         let encoded_len_name = Ident::from(format!("{}_DATA::ENCODED_LEN", self.name));

    //         if deser_vars.is_empty() {
    //             // struct has no fields
    //             quote!{
    //                 Some(Self::default())
    //             }
    //         } else {
    //             quote!{
    //                 let avail_len = _input.len();

    //                 //fast zero copy
    //                 let mut buf = Bytes::from(_input).into_buf();

    //                 // handle payload length truncuation due to empty fields
    //                 if avail_len < #encoded_len_name {
    //                     //copy available bytes into an oversized buffer filled with zeros
    //                     let mut payload_buf  = [0; #encoded_len_name];
    //                     payload_buf[0..avail_len].copy_from_slice(_input);
    //                     buf = Bytes::from(&payload_buf[..]).into_buf();
    //                 }

    //                 let mut _struct = Self::default();
    //                 #(#deser_vars)*
    //                 Some(_struct)
    //             }
    //         }
    // }

    ///Emit struct variables from arbitrary types
    fn emit_arbitrary_names(&self) -> TokenStream {
        let mut field_toks = TokenStream::new();
        self.fields
            .iter()
            .for_each(|field| {
                let name = field.emit_name();
                field_toks.extend(quote!{
                    #name: Arbitrary::arbitrary(_g),
                });
            });
        field_toks
    }


    /// Emit rust tokens for MavMessage
    /// Includes struct description, documentation and all `impl` blocks
    pub fn emit_rust(&self) -> TokenStream {
        let msg_name = self.emit_struct_name();
        let (name_types, msg_encoded_len) = self.emit_name_types();

        let deser_vars = self.emit_deserialize_vars();
        let serialize_vars = self.emit_serialize_vars();
        let description = self.emit_description();

        let test_name = format!("{}_test", self.name.to_lowercase());
        let test_name = syn::Ident::new(&test_name, self.name.span());

        let arbitrary_names = self.emit_arbitrary_names();

        quote!{
            #description
            #[derive(Debug, Clone, PartialEq, Default)]
            pub struct #msg_name {
                #(#name_types)*
            }

            impl #msg_name {
                pub const ENCODED_LEN: usize = #msg_encoded_len;

                fn deser(input: &[u8]) -> Result<Self, Error> {
                    let mut var = Self::default();
                    let mut _idx = 0;
                    #deser_vars
                    Ok(var)
                }

                fn ser(&self) -> Result<Vec<u8>, Error> {
                    let mut buf = vec![0;Self::ENCODED_LEN];
                    let mut _idx = 0;
                    #serialize_vars
                    Ok(buf[0.._idx].to_vec())
                }
            }

            #[cfg(test)]
            mod #test_name {
                use super::*;

                impl Arbitrary for #msg_name {
                    fn arbitrary<G: Gen>(_g: &mut G) -> Self {
                        #msg_name {
                            #arbitrary_names
                        }
                    }
                }

                #[quickcheck]
                fn qc_roundtrips(x: #msg_name) -> Result<TestResult, Error> {
                    let mut buf = x.ser()?;
                    let y = #msg_name::deser(&buf)?;
                    Ok(TestResult::from_bool(x == y))
                }
            }
        }
    }
}


#[derive(Debug, PartialEq, Clone, Default)]
pub struct MavField {
    pub mavtype: MavType,
    pub name: String,
    pub description: Option<String>,
    pub enumtype: Option<String>,
    pub display: Option<String>,
}

impl MavField {
    /// Emit rust name of a given field
    fn emit_name(&self) -> TokenStream {
        let name = self.name.clone();
        let name = syn::Ident::new(&name, name.span());
        quote!(#name)
    }

    /// Emit rust type of the field
    fn emit_type(&self) -> TokenStream {
        let mavtype = self.mavtype.rust_type();
        quote!(#mavtype)
    }

    /// Generate description for the given field
    fn emit_description(&self) -> TokenStream {
        if let Some(val) = self.description.clone() {
            let desc = format!("{}.",val);
            quote!{#[doc = #desc]}
        } else {
            quote!{}
        }
    }

    /// Combine rust name and type of a given field
    fn emit_name_type(&self) -> TokenStream {
        let name = self.emit_name();
        let fieldtype = self.emit_type(); 
        quote!(pub #name: #fieldtype,)
    }


    /// Emit serialization code
    fn rust_writer(&self) -> TokenStream {
        let varname = syn::Ident::new(&self.name, self.name.span());
        quote!{
            _idx += self.#varname.ser(&mut buf[_idx..])?;
        }
    }

    /// Emit deserialization code
    fn rust_reader(&self) -> TokenStream {
        let varname = syn::Ident::new(&self.name, self.name.span());
        let vartype = self.mavtype.rust_type();
        quote!{
            var.#varname = < #vartype >::deser(&input[_idx..])?;
            _idx += < #vartype >::element_size();
        }
    }


//     /// Emit reader
//     fn rust_reader(&self) -> Tokens {
//         let name = Ident::from("_struct.".to_string() + &self.name.clone());
//         let buf = Ident::from("buf");
//         if let Some(enum_name) = &self.enumtype {
//             if let Some(dsp) = &self.display {
//                 if dsp == "bitmask" {
//                     // bitflags
//                     let tmp = self.mavtype.rust_reader(Ident::from("let tmp"), buf.clone());
//                     let enum_name = Ident::from(enum_name.clone());
//                     quote!{
//                         #tmp
//                         #name = #enum_name::from_bits(tmp).expect("Unexpected enum value.");
//                     }

//                 } else {
//                     panic!("Display option not implemented");
//                 }
//             } else {
//                 // handle enum by FromPrimitive
//                 let tmp = self.mavtype.rust_reader(Ident::from("let tmp"), buf.clone());
//                 let val = Ident::from("from_".to_string() + &self.mavtype.rust_type());
//                 quote!(
//                     #tmp
//                     #name = FromPrimitive::#val(tmp).expect(&format!("Unexpected enum value {}.",tmp));
//                 )
//             }
//         } else {
//             self.mavtype.rust_reader(name, buf)
//         }
//     }
}

#[derive(Debug, PartialEq, Clone)]
pub enum MavType {
    UInt8MavlinkVersion,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Int8,
    Int16,
    Int32,
    Int64,
    Char,
    Float,
    Double,
    Array(Box<MavType>, usize),
}

impl Default for MavType {
    fn default() -> MavType {
        MavType::UInt8
    }
}

impl MavType {
    pub fn parse_type(s: &str) -> Option<MavType> {
        use self::MavType::*;
        match s {
            "uint8_t_mavlink_version" => Some(UInt8MavlinkVersion),
            "uint8_t" => Some(UInt8),
            "uint16_t" => Some(UInt16),
            "uint32_t" => Some(UInt32),
            "uint64_t" => Some(UInt64),
            "int8_t" => Some(Int8),
            "int16_t" => Some(Int16),
            "int32_t" => Some(Int32),
            "int64_t" => Some(Int64),
            "char" => Some(Char),
            "float" => Some(Float),
            "Double" => Some(Double),
            _ => {
                if s.ends_with("]") {
                    let start = s.find("[").unwrap();
                    let size = s[start + 1..(s.len() - 1)].parse::<usize>().unwrap();
                    let mtype = MavType::parse_type(&s[0..start]).unwrap();
                    Some(Array(Box::new(mtype), size))
                } else {
                    panic!("UNHANDLED {:?}", s);
                }
            }
        }
    }

//     /// Emit reader of a given type
//     pub fn rust_reader(&self, val: Ident, buf: Ident) -> Tokens {
//         use self::MavType::*;
//         match self.clone() {
//             Char => quote!{#val = #buf.get_u8() as char;},
//             UInt8 => quote!{#val = #buf.get_u8();},
//             UInt16 => quote!{#val = #buf.get_u16_le();},
//             UInt32 => quote!{#val = #buf.get_u32_le();},
//             UInt64 => quote!{#val = #buf.get_u64_le();},
//             UInt8MavlinkVersion => quote!{#val = #buf.get_u8();},
//             Int8 => quote!{#val = #buf.get_i8();},
//             Int16 => quote!{#val = #buf.get_i16_le();},
//             Int32 => quote!{#val = #buf.get_i32_le();},
//             Int64 => quote!{#val = #buf.get_i64_le();},
//             Float => quote!{#val = #buf.get_f32_le();},
//             Double => quote!{#val = #buf.get_f64_le();},
//             Array(t, size) => {
//                 if size > 32 {
//                     // it is a vector
//                     let r = t.rust_reader(Ident::from("let val"), buf.clone());
//                     quote!{
//                         for _ in 0..#size {
//                             #r
//                             #val.push(val);
//                         }
//                     }
//                 } else {
//                     // handle as a slice
//                     let r = t.rust_reader(Ident::from("let val"), buf.clone());
//                     quote!{
//                         for idx in 0..#val.len() {
//                             #r
//                             #val[idx] = val;
//                         }
//                     }
//                 }
//             }
//         }
//     }

    /// Emit serialization code for a given type
    /// ser(&self, output: &mut [u8]) -> Result<usize, Error>
    /// idx += #var.ser(buf[idx..])?;
    // pub fn rust_writer(&self, val: Ident, buf: Ident) -> Tokens {
    //     use self::MavType::*;
    //     match self.clone() {
    //         UInt8MavlinkVersion => quote!{
    //             #buf.put_u8(#val);
    //             },
    //         UInt8 => quote!{#buf.put_u8(#val);},
    //         Char => quote!{#buf.put_u8(#val as u8);},
    //         UInt16 => quote!{#buf.put_u16_le(#val);},
    //         UInt32 => quote!{#buf.put_u32_le(#val);},
    //         Int8 => quote!{#buf.put_i8(#val);},
    //         Int16 => quote!{#buf.put_i16_le(#val);},
    //         Int32 => quote!{#buf.put_i32_le(#val);},
    //         Float => quote!{#buf.put_f32_le(#val);},
    //         UInt64 => quote!{#buf.put_u64_le(#val);},
    //         Int64 => quote!{#buf.put_i64_le(#val);},
    //         Double => quote!{#buf.put_f64_le(#val);},
    //         Array(t,_size) => {
    //             let w = t.rust_writer(Ident::from("*val"), buf.clone());
    //             quote!{
    //                 #buf.put_u8(#val.len() as u8);
    //                 for val in &#val {
    //                     #w
    //                 }
    //             }
    //         },
    //     }
    // }

    /// Size of a given Mavtype
    fn len(&self) -> usize {
        use self::MavType::*;
        match self.clone() {
            UInt8MavlinkVersion | UInt8 | Int8 | Char => 1,
            UInt16 | Int16 => 2,
            UInt32 | Int32 | Float => 4,
            UInt64 | Int64 | Double => 8,
            Array(t, size) => t.len() * size,
        }
    }

    /// Used for ordering of types
    fn order_len(&self) -> usize {
        use self::MavType::*;
        match self.clone() {
            UInt8MavlinkVersion | UInt8 | Int8 | Char => 1,
            UInt16 | Int16 => 2,
            UInt32 | Int32 | Float => 4,
            UInt64 | Int64 | Double => 8,
            Array(t, _) => t.len(),
        }
    }

    /// Used for crc calculation
    pub fn primitive_type(&self) -> String {
        use self::MavType::*;
        match self.clone() {
            UInt8MavlinkVersion => "uint8_t".into(),
            UInt8 => "uint8_t".into(),
            Int8 => "int8_t".into(),
            Char => "char".into(),
            UInt16 => "uint16_t".into(),
            Int16 => "int16_t".into(),
            UInt32 => "uint32_t".into(),
            Int32 => "int32_t".into(),
            Float => "float".into(),
            UInt64 => "uint64_t".into(),
            Int64 => "int64_t".into(),
            Double => "double".into(),
            Array(t, _) => t.primitive_type(),
        }
    }

    /// Return rust equivalent of a given Mavtype
    /// Used for generating struct fields.
    pub fn rust_type(&self) -> TokenStream {
        use self::MavType::*;
        match self.clone() {
            UInt8 | UInt8MavlinkVersion => {
                quote!{u8}
            },
            Int8 => {
                quote!{i8}
            },
            Char => {
                quote!{char}
            },
            UInt16 => {
                quote!{u16}
            },
            Int16 => {
                quote!{i16}
            },
            UInt32 => {
                quote!{u32}
            },
            Int32 => {
                quote!{i32}
            },
            Float => {
                quote!{f32}
            },
            UInt64 => {
                quote!{u64}
            },
            Int64 => {
                quote!{i64}
            },
            Double => {
                quote!{f64}
            },
            Array(t, size) => {
                // if size > 32 {
                //     // we have to use a vector to make our lives easier
                //     let mavtype = t.rust_type();
                //     quote!{ SizedVec<  #mavtype, #size> }
                // } else {
                //     // we can use a slice, as Rust derives lot of thinsg for slices <= 32 elements
                //     let mavtype = t.rust_type();
                //     quote!{ [ #mavtype ; #size ]}
                // }
                let mavtype = t.rust_type();
                //quote!{ [ #mavtype ; #size ]}
                quote!{ ArrayVec < [ #mavtype ; #size ] > }
            },
        }
    }

    /// Compare two MavTypes
    pub fn compare(&self, other: &Self) -> Ordering {
        let len = self.order_len();
        (-(len as isize)).cmp(&(-(other.order_len() as isize)))
    }
}


#[derive(Debug, PartialEq, Clone, Default)]
pub struct MavEnum {
    pub name: String,
    pub description: Option<String>,
    pub entries: Vec<MavEnumEntry>,
    /// If contains Some, the string represents the type witdh for bitflags
    pub bitfield: Option<String>,
}

// impl MavEnum {
//     fn has_enum_values(&self) -> bool {
//         self.entries.iter().all(|x| x.value.is_some())
//     }

//     fn emit_defs(&self) -> Vec<Tokens> {
//         let mut cnt = 0;
//         self.entries
//             .iter()
//             .map(|enum_entry| {
//                 let name = Ident::from(enum_entry.name.clone());
//                 let value;
//                 if !self.has_enum_values() {
//                     value = Ident::from(cnt.to_string());
//                     cnt += 1;
//                 } else {
//                     value = Ident::from(enum_entry.value.unwrap().to_string());
//                 };
//                 if self.bitfield.is_some() {
//                     quote!(const #name = #value;)
//                 } else {
//                     quote!(#name = #value,)
//                 }
//             })
//             .collect::<Vec<Tokens>>()
//     }

//     fn emit_name(&self) -> Tokens {
//         let name = Ident::from(self.name.clone());
//         quote!(#name)
//     }

//     pub fn emit_rust(&self) -> Tokens {
//         let defs = self.emit_defs();
//         let default = Ident::from(self.entries[0].name.clone());
//         let enum_name = self.emit_name();

//         let enum_def;
//         if let Some(width) = self.bitfield.clone() {
//             let width = Ident::from(width);
//             enum_def = quote!{
//                 bitflags!{
//                     pub struct #enum_name: #width {
//                         #(#defs)*
//                     }
//                 }
//             };
//         } else {
//             enum_def = quote!{
//                 #[derive(Debug, Copy, Clone, PartialEq, FromPrimitive)]
//                 pub enum #enum_name {
//                     #(#defs)*
//                 }
//             };
//         }

//         quote!{
//             #enum_def

//             impl Default for #enum_name {
//                 fn default() -> Self {
//                     #enum_name::#default
//                 }
//             }
//         }
//     }
// }

#[derive(Debug, PartialEq, Clone, Default)]
pub struct MavEnumEntry {
    pub value: Option<i32>,
    pub name: String,
    pub description: Option<String>,
    pub params: Option<Vec<String>>,
}
